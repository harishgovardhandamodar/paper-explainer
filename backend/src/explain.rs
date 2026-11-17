//! Deterministic extraction + explanation engine.
//!
//! This is deliberately dependency-free (no LLM required): it reads the parsed
//! sections and uses linguistic cues to recover hypotheses, experiments,
//! findings and concepts. Curated papers bypass this module entirely. The
//! output is always a fully-populated `Extraction` so the frontend has a
//! consistent shape to render.

use crate::models::*;
use crate::parse::ParsedDocument;
use crate::util::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

#[derive(Debug, Clone, Default)]
pub struct Extraction {
    pub summary: String,
    pub tags: Vec<String>,
    pub hypotheses: Vec<Hypothesis>,
    pub experiments: Vec<Experiment>,
    pub findings: Vec<Finding>,
    pub diagrams: Vec<Diagram>,
    pub workflows: Vec<Workflow>,
    pub concepts: Vec<Concept>,
    pub notes: Vec<String>,
}

static HYPOTHESIS_CUES: &[&str] = &[
    "we hypothesize", "our hypothesis", "we argue", "we claim", "research question",
    "we ask", "we address", "we investigate", "we study", "fundamental question",
    "our key insight", "the key insight", "can we", "whether", "we propose", "we systematize",
    "our goal is", "we aim to", "we seek to", "we examine",
];
static FINDING_CUES: &[&str] = &[
    "we find", "we show", "results show", "the results", "we demonstrate", "answer to rq",
    "we observe", "our experiments", "outperform", "achieves", "reduces", "improves",
    "we reveal", "we establish", "we conclude",
];

static MODEL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:BERT(?:-Base|-Large)?|ViT(?:-Base)?|Qwen2?\.5?(?:-\d+(?:\.\d+)?B)?|LLaMA-?\d*|GPT-?\d|ResNet-?\d+|MobileNet|Phi-?\d?|Mistral|Gemma|RoBERTa|T5|GPT-2)\b").unwrap());
static DATASET_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:MNLI|QNLI|SST-2|RTE|CoLA|MRPC|CIFAR-?\d+|ImageNet|COCO|SQuAD|ADE20K|GLUE|WikiText|LAMBADA)\b").unwrap());
static METRIC_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:accuracy|top-1 accuracy|f1 ?score|precision|recall|perplexity|latency|throughput|bleu|rouge|auc|rel\.?black)\b").unwrap());
static TERM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:we (?:term|call|define|introduce)|called|termed|referred to as)\s+([A-Z][\w\-]{2,40}|[a-z][\w\-]{2,25})").unwrap());
static KNOWN_TERMS: &[(&str, &str, &str)] = &[
    ("TEE", "Trusted Execution Environment: isolated enclave that keeps secrets even when the OS is untrusted.", "primitive"),
    ("REE", "Rich Execution Environment: the untrusted accelerator side that runs offloaded computation.", "primitive"),
    ("obfuscation primitive", "A correct, cheap transform pair that disguises weights for offloading and restores the exact product inside the enclave.", "primitive"),
    ("surrogate model", "A model with the victim's architecture and comparable accuracy, built without exact parameter recovery.", "concept"),
    ("model extraction", "Recovering a model's functionality from observed outputs or exposed intermediate tensors.", "attack"),
];
static ACRONYM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b([A-Z]{2,6})\b").unwrap());
static RQ_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bRQ\s*(\d+)\s*[:.]\s*([^.]{10,160})").unwrap());

pub fn explain(doc: &ParsedDocument, paper_id: &str, url: &str) -> Extraction {
    let all_paras: Vec<(String, String)> = doc.sections.iter()
        .flat_map(|s| s.paragraphs.iter().map(move |p| (s.heading.clone(), p.clone())))
        .collect();

    let summary = build_summary(doc);
    let tags = build_tags(doc);
    let mut findings = extract_findings(&all_paras, paper_id);
    let mut hypotheses = extract_hypotheses(&all_paras, paper_id);
    let mut experiments = extract_experiments(doc, paper_id);
    let concepts = extract_concepts(doc, &all_paras, paper_id);

    link_entities(&mut hypotheses, &mut experiments, &mut findings);

    let mut diagrams = Vec::new();
    diagrams.push(at_a_glance_diagram(paper_id, doc, &hypotheses, &experiments, &findings));
    if let Some(e) = experiments.first() {
        diagrams.push(workflow_diagram(paper_id, e));
    }
    diagrams.push(traceability_diagram(paper_id, &hypotheses, &experiments, &findings));
    diagrams.push(concept_diagram(paper_id, &concepts));

    let workflows = build_workflows(paper_id, &experiments);

    let mut notes = Vec::new();
    if all_paras.is_empty() {
        notes.push("No section text was detected; explanations are based on the available text only.".into());
    }
    if hypotheses.is_empty() {
        notes.push("No explicit hypothesis statement was detected; hypotheses were inferred from the research questions.".into());
        hypotheses = infer_hypotheses(doc, paper_id);
    }
    if experiments.is_empty() {
        notes.push("No dedicated evaluation section was detected; a generic experiment was synthesized.".into());
        experiments.push(generic_experiment(doc, paper_id));
    }
    if findings.is_empty() {
        notes.push("No result sentences were detected; findings were inferred from the abstract.".into());
        findings = infer_findings(doc, paper_id);
    }

    // Re-link after inference.
    link_entities(&mut hypotheses, &mut experiments, &mut findings);

    let _ = url;

    Extraction { summary, tags, hypotheses, experiments, findings, diagrams, workflows, concepts, notes }
}

fn build_summary(doc: &ParsedDocument) -> String {
    let base = if doc.abstract_text.is_empty() {
        doc.sections.iter().take(2).flat_map(|s| s.paragraphs.iter()).take(2).cloned().collect::<Vec<_>>().join(" ")
    } else {
        doc.abstract_text.clone()
    };
    let simplified = simplify(&base);
    if simplified.is_empty() {
        format!("This paper, \"{}\", is added to the collection. Add notes or import a fuller version to generate a detailed explanation.", doc.title)
    } else {
        truncate(&simplified, 950)
    }
}

fn build_tags(doc: &ParsedDocument) -> Vec<String> {
    let mut tags: Vec<String> = doc.keywords.iter().take(8).cloned().collect();
    let text = format!("{} {}", doc.title, doc.sections.iter().map(|s| s.heading.clone()).collect::<Vec<_>>().join(" "));
    for m in MODEL_RE.find_iter(&text).map(|m| m.as_str()) { push_unique(&mut tags, m); }
    for d in DATASET_RE.find_iter(&text).map(|m| m.as_str()) { push_unique(&mut tags, d); }
    for term in ["security", "llm", "privacy", "systems", "machine learning", "attacks", "evaluation"] {
        if text.to_lowercase().contains(term) { push_unique(&mut tags, term); }
    }
    tags.truncate(12);
    tags
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    let s = clean(s);
    if !s.is_empty() && !v.iter().any(|x| x.eq_ignore_ascii_case(&s)) { v.push(s); }
}

fn extract_hypotheses(paras: &[(String, String)], paper_id: &str) -> Vec<Hypothesis> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (section, para) in paras {
        let lower = para.to_lowercase();
        if !HYPOTHESIS_CUES.iter().any(|c| lower.contains(c)) { continue; }
        let picked: String = sentences(para).into_iter()
            .filter(|s| HYPOTHESIS_CUES.iter().any(|c| s.to_lowercase().contains(c)))
            .take(2).collect::<Vec<_>>().join(" ");
        let statement = truncate(if picked.is_empty() { para } else { &picked }, 420);
        if statement.chars().count() < 40 || !seen.insert(statement.clone()) { continue; }
        let result_words = ["demonstrate", "we show", "we find", "results", "achieve", "reveal", "prove"];
        let status = if result_words.iter().any(|w| lower.contains(w)) {
            HypothesisStatus::Supported
        } else {
            HypothesisStatus::Open
        };
        out.push(Hypothesis {
            id: make_scoped_id("hyp", paper_id, &statement),
            statement: statement.clone(),
            simplified: truncate(&simplify(&statement), 320),
            status,
            section: Some(section.clone()),
            ..Default::default()
        });
        if out.len() >= 6 { break; }
    }
    out
}

fn infer_hypotheses(doc: &ParsedDocument, paper_id: &str) -> Vec<Hypothesis> {
    let mut statements = Vec::new();
    for m in RQ_RE.captures_iter(&doc.sections.iter().map(|s| s.paragraphs.join(" ")).collect::<Vec<_>>().join(" ")) {
        let q = format!("{}?", clean(&m[2]));
        statements.push(format!("The paper investigates: {}", truncate(&q, 250)));
    }
    if statements.is_empty() {
        statements.push(format!("The approach proposed in \"{}\" improves on prior methods.", doc.title));
    }
    statements.into_iter().take(5).map(|s| Hypothesis {
        id: make_scoped_id("hyp", paper_id, &s),
        statement: s.clone(), simplified: simplify(&s), status: HypothesisStatus::Open,
        ..Default::default()
    }).collect()
}

fn extract_findings(paras: &[(String, String)], paper_id: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut scored: Vec<(usize, &String, &String)> = paras.iter().map(|(s, p)| {
        let lower = p.to_lowercase();
        let cue = FINDING_CUES.iter().filter(|c| lower.contains(**c)).count();
        let nums = extract_numbers(p).len();
        (cue * 2 + nums, s, p)
    }).filter(|(score, _, _)| *score > 0).collect();
    scored.sort_by_key(|(score, _, _)| std::cmp::Reverse(*score));

    for (_, section, para) in scored {
        let best = sentences(para).into_iter()
            .find(|s| {
                let l = s.to_lowercase();
                FINDING_CUES.iter().any(|c| l.contains(*c)) || !extract_numbers(s).is_empty()
            })
            .unwrap_or_else(|| truncate(para, 260));
        let claim = truncate(&best, 340);
        if !seen.insert(claim.clone()) { continue; }
        let numbers = extract_numbers(&claim);
        let caveat = if ["we do not claim", "limited", "future work", "empirical", "out of scope", "attack-specific"].iter().any(|c| claim.to_lowercase().contains(c)) {
            Some("The authors qualify this claim in the paper (scope/limitations).".into())
        } else { None };
        out.push(Finding {
            id: make_scoped_id("find", paper_id, &claim),
            claim: claim.clone(),
            simplified: truncate(&simplify(&claim), 260),
            evidence: if numbers.is_empty() { format!("Reported in §{}", section) } else { numbers.join(" · ") },
            experiment_id: None,
            importance: if numbers.is_empty() { Importance::Medium } else { Importance::High },
            caveat,
        });
        if out.len() >= 10 { break; }
    }
    out
}

fn infer_findings(doc: &ParsedDocument, paper_id: &str) -> Vec<Finding> {
    let text = if doc.abstract_text.is_empty() {
        doc.sections.iter().flat_map(|s| s.paragraphs.iter()).take(3).cloned().collect::<Vec<_>>().join(" ")
    } else { doc.abstract_text.clone() };
    sentences(&text).into_iter().take(4).map(|s| Finding {
        id: make_scoped_id("find", paper_id, &s),
        claim: s.clone(), simplified: simplify(&s),
        evidence: extract_numbers(&s).join(" · "),
        experiment_id: None, importance: Importance::Medium, caveat: None,
    }).collect()
}

fn extract_experiments(doc: &ParsedDocument, paper_id: &str) -> Vec<Experiment> {
    let mut out = Vec::new();
    let mut candidates: Vec<_> = doc.sections.iter().filter(|s| {
        let h = s.heading.to_lowercase();
        h.contains("rq") || h.contains("experiment") || h.contains("evaluation")
            || h.contains("result") || h.contains("ablation") || h.contains("setup")
    }).collect();
    // Prefer the most detailed section when the same RQ is listed and elaborated.
    candidates.sort_by_key(|s| std::cmp::Reverse(s.paragraphs.iter().map(|p| p.len()).sum::<usize>()));
    let mut seen_rq: HashSet<String> = HashSet::new();

    for sec in candidates {
        let body = sec.paragraphs.join(" ");
        if let Some(m) = RQ_RE.captures(&body).or_else(|| RQ_RE.captures(&sec.heading)) {
            if !seen_rq.insert(m[1].to_string()) { continue; }
        }
        let rq = RQ_RE.captures(&body).or_else(|| RQ_RE.captures(&sec.heading));
        let question = rq.map(|m| format!("{}?", clean(&m[2])))
            .unwrap_or_else(|| truncate(&simplify(&sec.heading), 180));
        let name = if let Some(m) = RQ_RE.captures(&sec.heading).or_else(|| RQ_RE.captures(&body)) {
            format!("RQ{} · {}", &m[1], truncate(&clean(&m[2]), 90))
        } else { truncate(&simplify(&sec.heading), 80) };

        let design = truncate(&simplify(&body), 620);
        let setup = build_setup(&body);
        let steps = build_steps(&body, &sec.heading);
        let results: Vec<String> = sentences(&body).into_iter()
            .filter(|s| !extract_numbers(s).is_empty())
            .map(|s| truncate(&s, 220)).take(6).collect();
        let mermaid = steps_to_mermaid(&steps);

        out.push(Experiment {
            id: make_scoped_id("exp", paper_id, &name),
            name, question, design, setup, steps, results,
            mermaid: Some(mermaid), section: Some(sec.heading.clone()),
            ..Default::default()
        });
    }

    // Merge near-duplicate RQ sections (e.g. numbered heading + body heading).
    let mut dedup: Vec<Experiment> = Vec::new();
    for e in out {
        if dedup.iter().any(|x| x.name.eq_ignore_ascii_case(&e.name)) { continue; }
        dedup.push(e);
    }
    dedup.truncate(8);
    dedup
}

fn generic_experiment(doc: &ParsedDocument, paper_id: &str) -> Experiment {
    let body = doc.sections.iter().flat_map(|s| s.paragraphs.iter()).take(4).cloned().collect::<Vec<_>>().join(" ");
    let steps = build_steps(&body, "Experimental procedure");
    Experiment {
        id: make_scoped_id("exp", paper_id, "Evaluation"),
        name: "Evaluation".into(),
        question: format!("How was \"{}\" evaluated?", doc.title),
        design: truncate(&simplify(&body), 600),
        setup: build_setup(&body),
        mermaid: Some(steps_to_mermaid(&steps)),
        steps,
        ..Default::default()
    }
}

fn build_setup(body: &str) -> ExperimentSetup {
    let collect = |re: &Regex| -> Vec<String> {
        let mut v = Vec::new();
        for m in re.find_iter(body) { push_unique(&mut v, m.as_str()); }
        v
    };
    let mut metrics = collect(&METRIC_RE);
    let budget = Regex::new(r"(?i)(less than\s+)?\d+(?:\.\d+)?\s*%\s*of\s+(?:the\s+)?(?:original\s+)?training|1\s*%-task-data|\b\d+\s*%-budget").unwrap()
        .find(body).map(|m| clean(m.as_str()));
    ExperimentSetup {
        models: collect(&MODEL_RE),
        datasets: collect(&DATASET_RE),
        metrics: if metrics.is_empty() { vec!["task accuracy".into()] } else { metrics.drain(..).collect() },
        baselines: ["white-box", "black-box", "prior attack", "prior defense"].iter()
            .filter(|b| body.to_lowercase().contains(**b)).map(|b| b.to_string()).collect(),
        conditions: extract_numbers(body).into_iter().take(6).collect(),
        budget,
    }
}

fn build_steps(body: &str, heading: &str) -> Vec<WorkflowStep> {
    let picked = sentences(body).into_iter().take(4).collect::<Vec<_>>();
    if picked.is_empty() {
        return vec![WorkflowStep { id: "s1".into(), title: "Run the study".into(), detail: heading.into(), ..Default::default() }];
    }
    picked.into_iter().enumerate().map(|(i, s)| {
        let words: Vec<&str> = s.split_whitespace().take(6).collect();
        let title = if words.len() < 3 { format!("Step {}", i + 1) } else { words.join(" ") };
        WorkflowStep {
            id: format!("s{}", i + 1),
            title: truncate(&title, 48),
            detail: truncate(&simplify(&s), 240),
            inputs: if i == 0 { vec!["paper method".into()] } else { vec!["previous stage output".into()] },
            outputs: vec![if i == 3 { "reported result".into() } else { "intermediate artifact".into() }],
        }
    }).collect()
}

fn steps_to_mermaid(steps: &[WorkflowStep]) -> String {
    let mut s = String::from("flowchart TD\n");
    for (i, step) in steps.iter().enumerate() {
        s.push_str(&format!("  S{}[\"{}\"]\n", i + 1, mermaid_escape(&step.title)));
        if i > 0 { s.push_str(&format!("  S{} --> S{}\n", i, i + 1)); }
    }
    s.push_str(&format!("  S{} --> OUT[\"Result\"]\n", steps.len().max(1)));
    s
}

fn extract_concepts(doc: &ParsedDocument, paras: &[(String, String)], paper_id: &str) -> Vec<Concept> {
    let mut concepts: Vec<Concept> = Vec::new();
    let mut seen = HashSet::new();
    let text = paras.iter().map(|(_, p)| p.as_str()).collect::<Vec<_>>().join(" ");

    for caps in TERM_RE.captures_iter(&text) {
        let name = clean(&caps[1]);
        if name.chars().count() < 3 || STOP_WORDS.contains(&name.to_lowercase().as_str())
            || !seen.insert(name.to_lowercase()) { continue; }
        let context_start = caps.get(0).unwrap().start().saturating_sub(60);
        let context_end = (caps.get(0).unwrap().end() + 160).min(text.len());
        let context = text.get(context_start..context_end).unwrap_or("");
        concepts.push(Concept {
            id: make_scoped_id("concept", paper_id, &name),
            name: name.clone(),
            definition: truncate(&simplify(context), 220),
            category: categorize_concept(&name),
            aliases: Vec::new(),
        });
        if concepts.len() >= 12 { break; }
    }

    // Known vocabulary and named artifacts make the graph much more useful than
    // free-text term matching alone.
    for (name, definition, category) in KNOWN_TERMS {
        if !text.to_lowercase().contains(&name.to_lowercase()) { continue; }
        if !seen.insert(name.to_lowercase()) { continue; }
        concepts.push(Concept {
            id: make_scoped_id("concept", paper_id, name), name: (*name).into(),
            definition: (*definition).into(), category: (*category).into(), aliases: Vec::new(),
        });
    }
    let all_text = format!("{} {}", doc.title, text);
    for re in [&*MODEL_RE, &*DATASET_RE] {
        for m in re.find_iter(&all_text) {
            let name = clean(m.as_str());
            if !seen.insert(name.to_lowercase()) { continue; }
            let category = if MODEL_RE.is_match(&name) { "model" } else { "dataset" };
            concepts.push(Concept {
                id: make_scoped_id("concept", paper_id, &name), name: name.clone(),
                definition: format!("Named {category} used or discussed in this paper."),
                category: category.into(), aliases: Vec::new(),
            });
        }
    }

    if concepts.len() < 6 {
        for caps in ACRONYM_RE.captures_iter(&doc.title) {
            let name = caps[1].to_string();
            if ["LLM", "TEE", "REE", "GPU", "API", "NLI", "QA"].contains(&name.as_str()) { continue; }
            if !seen.insert(name.to_lowercase()) { continue; }
            concepts.push(Concept {
                id: make_scoped_id("concept", paper_id, &name), name: name.clone(),
                definition: format!("Acronym used in \"{}\".", doc.title), category: categorize_concept(&name), aliases: Vec::new(),
            });
        }
    }
    concepts.truncate(14);
    concepts
}

fn categorize_concept(name: &str) -> String {
    let l = name.to_lowercase();
    if ["attack", "collapse", "extraction", "adversary", "oracle"].iter().any(|k| l.contains(k)) { "attack".into() }
    else if ["mask", "perm", "scal", "primitiv", "mix", "obfuscat", "tee", "ree"].iter().any(|k| l.contains(k)) { "primitive".into() }
    else if ["accuracy", "f1", "latency", "metric", "rel"].iter().any(|k| l.contains(k)) { "metric".into() }
    else if ["bert", "vit", "qwen", "llama", "gpt"].iter().any(|k| l.contains(k)) { "model".into() }
    else { "concept".into() }
}

fn link_entities(hypotheses: &mut [Hypothesis], experiments: &mut [Experiment], findings: &mut [Finding]) {
    for h in hypotheses.iter_mut() {
        let hw = keyword_set(&format!("{} {}", h.statement, h.simplified));
        for f in findings.iter() {
            if overlap(&hw, &keyword_set(&f.claim)) >= 2 {
                h.evidence_ids.push(f.id.clone());
                if let Some(eid) = &f.experiment_id { push_unique(&mut h.experiment_ids, eid); }
            }
        }
        for e in experiments.iter() {
            if overlap(&hw, &keyword_set(&format!("{} {} {}", e.name, e.question, e.design))) >= 2 {
                push_unique(&mut h.experiment_ids, &e.id);
            }
        }
    }
    for e in experiments.iter_mut() {
        let ew = keyword_set(&format!("{} {} {}", e.name, e.question, e.design));
        for f in findings.iter_mut() {
            if overlap(&ew, &keyword_set(&f.claim)) >= 2 {
                f.experiment_id = Some(e.id.clone());
                push_unique(&mut e.finding_ids, &f.id);
            }
        }
        for h in hypotheses.iter() {
            if overlap(&ew, &keyword_set(&h.statement)) >= 2 { push_unique(&mut e.hypothesis_ids, &h.id); }
        }
    }
}

fn keyword_set(text: &str) -> HashSet<String> {
    text.to_lowercase().split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 4 && !STOP_WORDS.contains(w))
        .map(|w| w.trim_end_matches('s').to_string()).collect()
}

fn overlap(a: &HashSet<String>, b: &HashSet<String>) -> usize {
    a.intersection(b).count()
}

const STOP_WORDS: &[&str] = &["about", "after", "again", "against", "among", "because", "been", "before", "being", "between", "could", "does", "doing", "each", "from", "have", "into", "its", "more", "most", "other", "over", "same", "should", "some", "such", "than", "that", "their", "them", "then", "there", "these", "they", "this", "those", "through", "under", "using", "very", "what", "when", "where", "which", "while", "with", "would", "your"];

// ---------------------------------------------------------------------------
// Diagram builders
// ---------------------------------------------------------------------------

pub fn mermaid_escape(s: &str) -> String {
    s.replace('"', "#quot;").replace('\n', " ").replace('\r', " ").replace('|', "#124;")
}

fn at_a_glance_diagram(paper_id: &str, doc: &ParsedDocument, h: &[Hypothesis], e: &[Experiment], f: &[Finding]) -> Diagram {
    let mermaid = format!(
        "flowchart LR\n  Q[\"Question<br/>{}\"] --> H[\"{} hypotheses\"]\n  H --> M[\"Method<br/>{}\"]\n  M --> E[\"{} experiments\"]\n  E --> F[\"{} key findings\"]\n  F --> C[\"Conclusion\"]\n  classDef q fill:#eef2ff,stroke:#6366f1,color:#312e81;\n  classDef r fill:#ecfdf5,stroke:#10b981,color:#065f46;\n  class Q,H q;\n  class E,F,C r;\n",
        mermaid_escape(&truncate(&doc.title, 46)), h.len(), mermaid_escape(&truncate(&doc.title.split_whitespace().take(3).collect::<Vec<_>>().join(" "), 34)), e.len(), f.len()
    );
    Diagram {
        id: make_scoped_id("dia", paper_id, "paper-at-a-glance"),
        title: "Paper at a glance".into(),
        caption: "The reasoning spine: question → hypotheses → method → experiments → findings.".into(),
        mermaid, kind: DiagramKind::Other, paper_id: paper_id.into(),
    }
}

fn workflow_diagram(paper_id: &str, exp: &Experiment) -> Diagram {
    let mermaid = exp.mermaid.clone().unwrap_or_else(|| "flowchart TD\n  A[\"Start\"] --> B[\"End\"]".into());
    Diagram {
        id: make_scoped_id("dia", paper_id, &format!("{}-workflow", exp.id)),
        title: format!("Recreated workflow · {}", truncate(&exp.name, 46)),
        caption: "A step-by-step reconstruction of the procedure described in the paper.".into(),
        mermaid, kind: DiagramKind::Workflow, paper_id: paper_id.into(),
    }
}

fn traceability_diagram(paper_id: &str, h: &[Hypothesis], e: &[Experiment], f: &[Finding]) -> Diagram {
    let mut s = String::from("flowchart LR\n");
    for (i, x) in h.iter().take(4).enumerate() {
        s.push_str(&format!("  H{}[\"H{} {}\"]\n", i, i + 1, mermaid_escape(&truncate(&x.simplified, 42))));
    }
    for (i, x) in e.iter().take(4).enumerate() {
        s.push_str(&format!("  E{}[\"E{} {}\"]\n", i, i + 1, mermaid_escape(&truncate(&x.name, 38))));
    }
    for (i, x) in f.iter().take(4).enumerate() {
        s.push_str(&format!("  F{}[\"F{} {}\"]\n", i, i + 1, mermaid_escape(&truncate(&x.simplified, 38))));
    }
    for i in 0..h.len().min(4) { s.push_str(&format!("  H{} --> E{}\n", i, i.min(e.len().saturating_sub(1)))); }
    for i in 0..e.len().min(4) { s.push_str(&format!("  E{} --> F{}\n", i, i.min(f.len().saturating_sub(1)))); }
    s.push_str("  classDef h fill:#eef2ff,stroke:#6366f1;\n  classDef f fill:#ecfdf5,stroke:#10b981;\n");
    for (i, _) in h.iter().take(4).enumerate() { s.push_str(&format!("  class H{} h;\n", i)); }
    for (i, _) in f.iter().take(4).enumerate() { s.push_str(&format!("  class F{} f;\n", i)); }
    Diagram {
        id: make_scoped_id("dia", paper_id, "traceability"),
        title: "Hypothesis → experiment → finding".into(),
        caption: "Traceability map: which experiments test which hypotheses, and what they found.".into(),
        mermaid: s, kind: DiagramKind::Other, paper_id: paper_id.into(),
    }
}

fn concept_diagram(paper_id: &str, concepts: &[Concept]) -> Diagram {
    let mut s = String::from("flowchart TD\n");
    for (i, c) in concepts.iter().take(8).enumerate() {
        s.push_str(&format!("  C{}[\"{}\"]\n", i, mermaid_escape(&truncate(&c.name, 26))));
        if i > 0 { s.push_str(&format!("  C0 --- C{}\n", i)); }
    }
    Diagram {
        id: make_scoped_id("dia", paper_id, "concept-map"),
        title: "Concept map".into(),
        caption: "The vocabulary introduced or used by this paper.".into(),
        mermaid: s, kind: DiagramKind::Taxonomy, paper_id: paper_id.into(),
    }
}

fn build_workflows(paper_id: &str, experiments: &[Experiment]) -> Vec<Workflow> {
    experiments.iter().take(3).map(|e| Workflow {
        id: make_scoped_id("wf", paper_id, &e.name),
        title: format!("Recreated workflow · {}", truncate(&e.name, 50)),
        description: truncate(&e.design, 340),
        steps: e.steps.clone(),
        mermaid: e.mermaid.clone().unwrap_or_default(),
        paper_id: paper_id.into(),
        category: "evaluation".into(),
    }).collect()
}

#[allow(dead_code)]
fn unused_map(_: &HashMap<String, String>) {}
