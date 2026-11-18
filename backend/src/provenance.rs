//! Per-item provenance, extraction rules and confidence.
//!
//! Every explained item (summary, hypothesis, experiment, finding, diagram,
//! workflow, concept, and each AI insight) carries a badge saying where it came
//! from, the named rule that produced it, and a confidence score. Confidence
//! describes how strong the extraction signal was — a number-backed finding
//! scores above a loose cue phrase — and is **not** a probability that the
//! claim is true. Humans can verify, re-badge or override any item; those edits
//! are marked `overridden` and survive refresh/re-import.

use crate::models::*;
use serde::Serialize;
use std::collections::HashSet;

/// Key for the whole-paper plain-language summary.
pub const SUMMARY_KEY: &str = "summary";

const ENHANCEMENT_FIELDS: &[&str] = &["tldr", "explanation", "how_it_works"];
const ENHANCEMENT_LISTS: &[&str] = &["key_insights", "caveats"];

fn entry(kind: ProvenanceKind, rule: &str, confidence: f32, section: Option<String>) -> Provenance {
    Provenance {
        kind,
        origin: kind,
        rule: rule.into(),
        confidence: confidence.clamp(0.0, 1.0),
        section,
        ..Default::default()
    }
}

fn curated_entry(rule: &str, section: Option<String>) -> Provenance {
    entry(ProvenanceKind::Curated, rule, 0.94, section)
}

fn summary_prov(paper: &Paper) -> Provenance {
    if paper.source == PaperSource::Curated {
        curated_entry("curated_seed", None)
    } else if !paper.abstract_text.trim().is_empty() {
        entry(ProvenanceKind::Heuristic, "abstract_simplify", 0.72, None)
    } else {
        entry(ProvenanceKind::Heuristic, "section_fallback", 0.45, None)
    }
}

fn hypothesis_prov(paper: &Paper, h: &Hypothesis) -> Provenance {
    if paper.source == PaperSource::Curated {
        return curated_entry("curated_seed", h.section.clone());
    }
    let lower = h.statement.to_lowercase();
    if lower.starts_with("the paper investigates:") || lower.starts_with("the approach proposed in") {
        entry(ProvenanceKind::Heuristic, "rq_inference", 0.45, h.section.clone())
    } else if lower.contains("research question") || lower.contains("we hypothesize") || lower.contains("our hypothesis") {
        entry(ProvenanceKind::Heuristic, "hypothesis_cue", 0.78, h.section.clone())
    } else if h.section.is_some() {
        entry(ProvenanceKind::Heuristic, "claim_phrase", 0.62, h.section.clone())
    } else {
        entry(ProvenanceKind::Heuristic, "claim_phrase", 0.55, h.section.clone())
    }
}

fn finding_prov(paper: &Paper, f: &Finding) -> Provenance {
    if paper.source == PaperSource::Curated {
        return curated_entry("curated_seed", None);
    }
    let mut prov = if f.evidence.starts_with("Reported in") {
        entry(ProvenanceKind::Heuristic, "section_reference", 0.5, None)
    } else if f.evidence.chars().any(|c| c.is_ascii_digit()) {
        entry(ProvenanceKind::Heuristic, "finding_numbers", 0.8, None)
    } else {
        entry(ProvenanceKind::Heuristic, "finding_cue", 0.6, None)
    };
    if f.caveat.is_some() {
        prov.note = "The extractor detected an author caveat on this claim.".into();
    }
    prov
}

fn experiment_prov(paper: &Paper, e: &Experiment) -> Provenance {
    if paper.source == PaperSource::Curated {
        return curated_entry("curated_seed", e.section.clone());
    }
    let (rule, conf) = if e.name == "Evaluation" && e.section.is_none() {
        ("generic_experiment", 0.38)
    } else if e.name.starts_with("RQ") {
        ("rq_section", 0.74)
    } else {
        ("experiment_section", 0.68)
    };
    entry(ProvenanceKind::Heuristic, rule, conf, e.section.clone())
}

fn diagram_prov(paper: &Paper, d: &Diagram) -> Provenance {
    if paper.source == PaperSource::Curated {
        curated_entry("curated_seed", None)
    } else {
        let mut prov = entry(ProvenanceKind::Heuristic, "diagram_template", 0.55, None);
        prov.note = format!("Template-generated {:?} diagram.", d.kind);
        prov
    }
}

fn workflow_prov(paper: &Paper, _w: &Workflow) -> Provenance {
    if paper.source == PaperSource::Curated {
        curated_entry("curated_seed", None)
    } else {
        entry(ProvenanceKind::Heuristic, "workflow_from_steps", 0.6, None)
    }
}

fn concept_prov(paper: &Paper, c: &Concept) -> Provenance {
    if paper.source == PaperSource::Curated {
        return curated_entry("curated_seed", None);
    }
    let (rule, conf) = if c.definition.starts_with("Named model") {
        ("model_match", 0.72)
    } else if c.definition.starts_with("Named dataset") {
        ("dataset_match", 0.72)
    } else if c.definition.starts_with("Acronym used in") {
        ("title_acronym", 0.4)
    } else if c.definition.chars().count() > 80 {
        ("term_pattern", 0.62)
    } else {
        ("known_vocabulary", 0.78)
    };
    entry(ProvenanceKind::Heuristic, rule, conf, None)
}

fn ai_entry(enhancement: &PaperEnhancement) -> Provenance {
    Provenance {
        kind: ProvenanceKind::Ai,
        origin: ProvenanceKind::Ai,
        rule: "llm_enhance".into(),
        confidence: 0.55,
        section: None,
        generator: if enhancement.served_model.is_empty() {
            enhancement.requested_model.clone()
        } else {
            enhancement.served_model.clone()
        },
        note: "AI-written and unverified; check the source paper.".into(),
        ..Default::default()
    }
}

/// Keys for the AI insight entries present when an enhancement exists.
pub fn enhancement_keys() -> Vec<String> {
    let mut keys: Vec<String> = ENHANCEMENT_FIELDS.iter().map(|f| format!("enhancement:{f}")).collect();
    for field in ENHANCEMENT_LISTS {
        // Counts are paper-specific, so callers use `enhancement_list_keys` for
        // exact keys; these bare prefixes are used for pruning checks only.
        keys.push(format!("enhancement:{field}"));
    }
    keys
}

fn enhancement_list_keys(field: &str, count: usize) -> Vec<String> {
    (0..count).map(|i| format!("enhancement:{field}:{i}")).collect()
}

/// Derive provenance for every currently present item on one paper.
pub fn derive_all(paper: &Paper) -> Vec<(String, Provenance)> {
    let mut out = Vec::new();
    out.push((SUMMARY_KEY.into(), summary_prov(paper)));
    for h in &paper.hypotheses { out.push((h.id.clone(), hypothesis_prov(paper, h))); }
    for e in &paper.experiments { out.push((e.id.clone(), experiment_prov(paper, e))); }
    for f in &paper.findings { out.push((f.id.clone(), finding_prov(paper, f))); }
    for d in &paper.diagrams { out.push((d.id.clone(), diagram_prov(paper, d))); }
    for w in &paper.workflows { out.push((w.id.clone(), workflow_prov(paper, w))); }
    for c in &paper.concepts { out.push((c.id.clone(), concept_prov(paper, c))); }
    if let Some(enh) = &paper.enhancement {
        for field in ENHANCEMENT_FIELDS {
            out.push((format!("enhancement:{field}"), ai_entry(enh)));
        }
        out.extend(enhancement_list_keys("key_insights", enh.key_insights.len())
            .into_iter().map(|k| (k, ai_entry(enh))));
        out.extend(enhancement_list_keys("caveats", enh.caveats.len())
            .into_iter().map(|k| (k, ai_entry(enh))));
    }
    out
}

/// Fill missing entries and drop stale, never-edited ones. Returns the number
/// of entries added. Existing entries (including human overrides) are kept.
pub fn backfill_paper(paper: &mut Paper) -> usize {
    let derived = derive_all(paper);
    let expected: HashSet<String> = derived.iter().map(|(k, _)| k.clone()).collect();

    paper.item_provenance.retain(|k, v| expected.contains(k) || v.overridden);

    let mut added = 0;
    for (key, prov) in derived {
        paper.item_provenance.entry(key).or_insert_with(|| {
            added += 1;
            prov
        });
    }
    for prov in paper.item_provenance.values_mut() {
        if prov.verified_at.is_some() {
            prov.kind = ProvenanceKind::Verified;
        }
    }
    added
}

/// Backfill a whole collection. Used at startup so older stored papers gain
/// provenance without any user action.
pub fn backfill(papers: &mut [Paper]) -> usize {
    papers.iter_mut().map(backfill_paper).sum()
}

/// Add or refresh AI-sourced entries after enhancement generation or removal.
/// Human-overridden entries are left alone so a model rerun cannot silently
/// replace a verified insight.
pub fn refresh_enhancement_provenance(paper: &mut Paper) {
    if let Some(enh) = &paper.enhancement {
        let mut keys: Vec<String> = ENHANCEMENT_FIELDS.iter().map(|f| format!("enhancement:{f}")).collect();
        keys.extend(enhancement_list_keys("key_insights", enh.key_insights.len()));
        keys.extend(enhancement_list_keys("caveats", enh.caveats.len()));
        for key in keys {
            if paper.item_provenance.get(&key).is_some_and(|p| p.overridden) { continue; }
            paper.item_provenance.insert(key, ai_entry(enh));
        }
    } else {
        paper.item_provenance.retain(|k, p| !k.starts_with("enhancement:") || p.overridden);
    }
}

/// A short, human-readable type name inferred from an item key.
pub fn item_type(key: &str) -> &'static str {
    if key == SUMMARY_KEY { "summary" }
    else if key.starts_with("enhancement:") { "AI insight" }
    else if key.starts_with("hyp:") { "hypothesis" }
    else if key.starts_with("find:") { "finding" }
    else if key.starts_with("exp:") { "experiment" }
    else if key.starts_with("dia:") { "diagram" }
    else if key.starts_with("wf:") { "workflow" }
    else if key.starts_with("concept:") { "concept" }
    else { "item" }
}

fn clip(s: &str, max: usize) -> String {
    let clean = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= max { return clean; }
    let cut: String = clean.chars().take(max.saturating_sub(1)).collect();
    format!("{cut}…")
}

fn label_for(paper: &Paper, key: &str) -> String {
    if key == SUMMARY_KEY { return clip(&paper.summary, 120); }
    if let Some(rest) = key.strip_prefix("enhancement:") {
        if let Some(enh) = &paper.enhancement {
            let value = match rest {
                "tldr" => enh.tldr.clone(),
                "explanation" => enh.explanation.clone(),
                "how_it_works" => enh.how_it_works.clone(),
                other => {
                    let (field, idx) = other.rsplit_once(':').unwrap_or((other, "0"));
                    let i = idx.parse::<usize>().unwrap_or(0);
                    match field {
                        "key_insights" => enh.key_insights.get(i).cloned().unwrap_or_default(),
                        "caveats" => enh.caveats.get(i).cloned().unwrap_or_default(),
                        _ => String::new(),
                    }
                }
            };
            return clip(&value, 120);
        }
    }
    let value = if let Some(h) = paper.hypotheses.iter().find(|h| h.id == key) { h.statement.clone() }
        else if let Some(f) = paper.findings.iter().find(|f| f.id == key) { f.claim.clone() }
        else if let Some(e) = paper.experiments.iter().find(|e| e.id == key) {
            if e.question.is_empty() { e.name.clone() } else { format!("{} — {}", e.name, e.question) }
        }
        else if let Some(d) = paper.diagrams.iter().find(|d| d.id == key) { d.title.clone() }
        else if let Some(w) = paper.workflows.iter().find(|w| w.id == key) { w.title.clone() }
        else if let Some(c) = paper.concepts.iter().find(|c| c.id == key) { c.name.clone() }
        else { key.into() };
    clip(&value, 120)
}

/// One row in the collection-wide provenance view.
#[derive(Debug, Clone, Serialize)]
pub struct ProvenanceItemView {
    pub key: String,
    pub paper_id: String,
    pub paper_title: String,
    pub item_type: String,
    pub label: String,
    pub badge: ProvenanceKind,
    pub origin: ProvenanceKind,
    pub rule: String,
    pub generator: String,
    pub confidence: f32,
    pub section: Option<String>,
    pub verified: bool,
    pub note: String,
    pub overridden: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProvenanceStats {
    pub items: usize,
    pub verified: usize,
    pub curated: usize,
    pub heuristic: usize,
    pub ai: usize,
    pub manual: usize,
    pub low_confidence: usize,
    pub average_confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProvenanceReport {
    pub items: Vec<ProvenanceItemView>,
    pub stats: ProvenanceStats,
}

/// Aggregate all papers into one reviewable list plus roll-up statistics.
pub fn report(papers: &[Paper]) -> ProvenanceReport {
    let mut items = Vec::new();
    for paper in papers {
        for (key, prov) in &paper.item_provenance {
            items.push(ProvenanceItemView {
                key: key.clone(),
                paper_id: paper.id.clone(),
                paper_title: paper.title.clone(),
                item_type: item_type(key).into(),
                label: label_for(paper, key),
                badge: prov.kind,
                origin: prov.origin,
                rule: prov.rule.clone(),
                generator: prov.generator.clone(),
                confidence: prov.confidence,
                section: prov.section.clone(),
                verified: prov.verified_at.is_some(),
                note: prov.note.clone(),
                overridden: prov.overridden,
            });
        }
    }
    items.sort_by(|a, b| {
        a.paper_title.cmp(&b.paper_title)
            .then(a.item_type.cmp(&b.item_type))
            .then(a.key.cmp(&b.key))
    });

    let stats = ProvenanceStats {
        items: items.len(),
        verified: items.iter().filter(|i| i.verified).count(),
        curated: items.iter().filter(|i| i.badge == ProvenanceKind::Curated).count(),
        heuristic: items.iter().filter(|i| i.badge == ProvenanceKind::Heuristic).count(),
        ai: items.iter().filter(|i| i.badge == ProvenanceKind::Ai).count(),
        manual: items.iter().filter(|i| i.badge == ProvenanceKind::Manual).count(),
        low_confidence: items.iter().filter(|i| i.confidence < 0.5).count(),
        average_confidence: if items.is_empty() { 0.0 } else {
            items.iter().map(|i| i.confidence).sum::<f32>() / items.len() as f32
        },
    };
    ProvenanceReport { items, stats }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Concept, Experiment, Finding, Hypothesis};

    fn synthetic_paper() -> Paper {
        let mut p = Paper::default();
        p.id = "p1".into();
        p.title = "A Synthetic Paper".into();
        p.source = PaperSource::Heuristic;
        p.abstract_text = "We study a thing.".into();
        p.hypotheses.push(Hypothesis {
            id: "hyp:p1:we-hypothesize-x".into(),
            statement: "We hypothesize that X improves Y under Z.".into(),
            section: Some("Introduction".into()),
            ..Default::default()
        });
        p.experiments.push(Experiment {
            id: "exp:p1:rq1-results".into(),
            name: "RQ1 · Results".into(),
            section: Some("Evaluation".into()),
            ..Default::default()
        });
        p.findings.push(Finding {
            id: "find:p1:12-improvement".into(),
            claim: "X improves Y by 12%.".into(),
            evidence: "12%".into(),
            ..Default::default()
        });
        p.concepts.push(Concept {
            id: "concept:p1:tee".into(),
            name: "TEE".into(),
            definition: "Trusted Execution Environment: isolated enclave that keeps secrets safe.".into(),
            category: "primitive".into(),
            aliases: Vec::new(),
        });
        p
    }

    #[test]
    fn backfill_assigns_provenance_to_every_item() {
        let mut p = synthetic_paper();
        let added = backfill_paper(&mut p);
        assert_eq!(added, 5); // summary + hypothesis + experiment + finding + concept
        assert_eq!(p.item_provenance[SUMMARY_KEY].rule, "abstract_simplify");
        assert_eq!(p.item_provenance["hyp:p1:we-hypothesize-x"].rule, "hypothesis_cue");
        assert_eq!(p.item_provenance["find:p1:12-improvement"].rule, "finding_numbers");
        assert_eq!(p.item_provenance["exp:p1:rq1-results"].rule, "rq_section");
        assert!(p.item_provenance["concept:p1:tee"].confidence > 0.7);
    }

    #[test]
    fn curated_paper_items_are_curated() {
        let mut p = synthetic_paper();
        p.source = PaperSource::Curated;
        backfill_paper(&mut p);
        assert!(p.item_provenance.values().all(|v| v.kind == ProvenanceKind::Curated));
        assert!(p.item_provenance.values().all(|v| v.rule == "curated_seed"));
    }

    #[test]
    fn overridden_entries_survive_rebackfill_and_stale_entries_are_pruned() {
        let mut p = synthetic_paper();
        backfill_paper(&mut p);
        let key = "find:p1:12-improvement";
        let prov = p.item_provenance.get_mut(key).unwrap();
        prov.overridden = true;
        prov.kind = ProvenanceKind::Verified;
        prov.origin = ProvenanceKind::Heuristic;
        prov.note = "Checked against Table 2".into();
        p.item_provenance.insert("hyp:p1:old-item".into(), Provenance::default());

        backfill_paper(&mut p);
        let prov = &p.item_provenance[key];
        assert_eq!(prov.kind, ProvenanceKind::Verified);
        assert_eq!(prov.origin, ProvenanceKind::Heuristic);
        assert_eq!(prov.note, "Checked against Table 2");
        assert!(!p.item_provenance.contains_key("hyp:p1:old-item"));
    }

    #[test]
    fn ai_entries_follow_enhancement_lifecycle() {
        let mut p = synthetic_paper();
        backfill_paper(&mut p);
        p.enhancement = Some(PaperEnhancement {
            tldr: "A short version".into(),
            key_insights: vec!["Insight one".into(), "Insight two".into()],
            served_model: "qwen3.8:27b".into(),
            ..Default::default()
        });
        refresh_enhancement_provenance(&mut p);
        assert_eq!(p.item_provenance["enhancement:tldr"].kind, ProvenanceKind::Ai);
        assert_eq!(p.item_provenance["enhancement:tldr"].generator, "qwen3.8:27b");
        assert_eq!(p.item_provenance["enhancement:key_insights:1"].rule, "llm_enhance");

        p.enhancement = None;
        refresh_enhancement_provenance(&mut p);
        assert!(!p.item_provenance.keys().any(|k| k.starts_with("enhancement:")));
    }

    #[test]
    fn report_aggregates_items_and_stats() {
        let mut p = synthetic_paper();
        backfill_paper(&mut p);
        let report = report(&[p]);
        assert_eq!(report.stats.items, 5);
        assert_eq!(report.stats.heuristic, 5);
        assert!(report.stats.average_confidence > 0.0 && report.stats.average_confidence <= 1.0);
        assert!(report.items.iter().any(|i| i.item_type == "finding" && i.label.contains("12%")));
    }
}
