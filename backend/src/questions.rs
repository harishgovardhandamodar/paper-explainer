//! Deterministic mining of reviewable open-question candidates.
//!
//! No model is called here. Candidates come from limitation / future-work /
//! discussion / conclusion sections, finding caveats, unresolved hypotheses and
//! extraction notes. Every candidate keeps its source sentence so a person can
//! judge whether the suggested question is worth promoting to a follow-up.

use crate::models::{OpenQuestion, Paper};
use crate::util::{capitalize_first, make_scoped_id, prose, truncate};
use regex::Regex;
use std::sync::LazyLock;

static FOCUS_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(limitation|future work|future research|open question|open problem|open challenge|threats? to validity|unresolved|remaining challenge|what .* (does not|do not|cannot) .* show)").unwrap()
});
static CONTEXT_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(discussion|conclusion|outlook|closing remarks|general discussion)").unwrap()
});

static FUTURE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(future work|future research|we leave|left to future|remains? (?:to be|an open)|open question|open problem|we plan|promising direction|could be extended|worth (?:exploring|investigating)|next step)\b").unwrap()
});
static UNCERTAINTY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(unclear|uncertain|remains? unknown|it is not known|we do not know|we cannot (?:tell|know|determine)|not yet (?:known|established)|open (?:problem|question))\b").unwrap()
});
static LIMITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(limitation|limited (?:by|to)|limits? (?:our|the|this)|restricted to|does not scale|doesn't scale|constraint|caveat|threat to validity|only (?:evaluated|tested|considered)|small (?:sample|scale))\b").unwrap()
});
static NEGATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\b(?:we|this work|our (?:approach|method|analysis|study|evaluation))\s+(?:do not|does not|did not|don't|doesn't|didn't|cannot|can't|fail to|failed to|ignore|lack)\b|\b(?:not address(?:ed)?|not consider(?:ed)?|not covered|beyond the scope|no (?:evaluation|analysis|comparison|discussion))\b)").unwrap()
});
static ASSUMPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(we assume|we assumed|assum(?:e|es|ing|ption)|under the assumption|relies? on|requires? that|we require)\b").unwrap()
});

static WE_NEG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*we\s+(?:do not|don't|does not|doesn't|did not|didn't)\s+(.+)$").unwrap()
});
static WE_ASSUME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*we\s+(?:assume|assumed|require|required)\s+(.+)$").unwrap()
});
static WE_ONLY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*we\s+(?:only|just)\s+(.+)$").unwrap()
});

/// Split prose into sentences without a lookbehind (the regex crate has none).
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut after_punct = false;
    for ch in text.chars() {
        if after_punct && ch.is_whitespace() {
            let s = cur.trim();
            if !s.is_empty() { out.push(s.to_string()); }
            cur.clear();
            after_punct = false;
            continue;
        }
        cur.push(ch);
        after_punct = matches!(ch, '.' | '!' | '?');
    }
    let tail = cur.trim();
    if !tail.is_empty() { out.push(tail.to_string()); }
    out
}

fn detect_cue(sentence: &str) -> Option<(&'static str, &'static str)> {
    if sentence.trim_end().ends_with('?') { return Some(("question", "high")); }
    if FUTURE_RE.is_match(sentence) { return Some(("future_work", "high")); }
    if UNCERTAINTY_RE.is_match(sentence) { return Some(("uncertainty", "high")); }
    if LIMITATION_RE.is_match(sentence) { return Some(("limitation", "medium")); }
    if NEGATION_RE.is_match(sentence) { return Some(("negation", "medium")); }
    if ASSUMPTION_RE.is_match(sentence) { return Some(("assumption", "low")); }
    None
}

/// Build a short, reviewable question. Targeted rewrites handle the most
/// common declarative shapes; everything else gets a cue-specific stem that
/// keeps the full source sentence visible for editing.
fn suggest_question(context: &str, cue: &str) -> String {
    let c = context.trim().trim_end_matches(['.', '!', '?']).trim();
    if cue == "question" {
        let q = if c.ends_with('?') { c.to_string() } else { format!("{c}?") };
        return capitalize_first(&q);
    }

    let targeted = if let Some(caps) = WE_NEG_RE.captures(c) {
        Some(format!("What happens if we {}?", caps[1].trim()))
    } else if let Some(caps) = WE_ASSUME_RE.captures(c) {
        Some(format!("What if the assumption that {} does not hold?", caps[1].trim()))
    } else if let Some(caps) = WE_ONLY_RE.captures(c) {
        Some(format!("Does the result hold without restricting to {}?", caps[1].trim()))
    } else {
        None
    };
    if let Some(q) = targeted { return q; }

    let lower = c.to_lowercase();
    let q = if lower.contains("future work") || lower.contains("future research") {
        format!("How could this be explored in future work: {c}?")
    } else {
        match cue {
            "future_work" => format!("Which follow-up would advance this: {c}?"),
            "uncertainty" => format!("How could we resolve this: {c}?"),
            "limitation" => format!("Does this still hold when: {c}?"),
            "negation" => format!("What if we remove this constraint: {c}?"),
            "assumption" => format!("What if this premise fails: {c}?"),
            "caveat" => format!("Under which conditions does this still hold: {c}?"),
            "unresolved" => format!("What evidence would settle this: {c}?"),
            _ => format!("What should we check here: {c}?"),
        }
    };
    capitalize_first(&q)
}

fn dedup_key(s: &str) -> String {
    let key: String = s.to_lowercase()
        .chars().filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
        .collect();
    let key = crate::util::clean(&key);
    key.chars().take(90).collect()
}

struct Candidate {
    paper_id: String,
    source: String,
    cue: String,
    context: String,
    text: String,
    confidence: String,
    target: String,
}

impl Candidate {
    fn into_question(self, seq: usize) -> OpenQuestion {
        OpenQuestion {
            id: make_scoped_id("oq", &self.paper_id, &format!("{}-{}", seq, self.context)),
            paper_id: self.paper_id,
            source: self.source,
            cue: self.cue,
            context: self.context,
            text: self.text,
            confidence: self.confidence,
            target: self.target,
        }
    }
}

fn push_candidate(
    out: &mut Vec<Candidate>, seen: &mut std::collections::HashSet<String>,
    paper_id: &str, source: &str, cue: &str, context: &str, confidence: &str, target: &str,
) {
    let context = truncate(&prose(context), 320);
    if context.chars().count() < 30 { return; }
    let key = dedup_key(&context);
    if !seen.insert(key) { return; }
    out.push(Candidate {
        paper_id: paper_id.to_string(),
        source: source.to_string(),
        cue: cue.to_string(),
        context: context.clone(),
        text: suggest_question(&context, cue),
        confidence: confidence.to_string(),
        target: target.to_string(),
    });
}

/// Extract reviewable open-question candidates from a paper's deterministic
/// extraction. Results are sorted by cue confidence and capped at 40.
pub fn extract(paper: &Paper) -> Vec<OpenQuestion> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for section in &paper.sections {
        let focus = FOCUS_HEADING_RE.is_match(&section.heading);
        let contextual = CONTEXT_HEADING_RE.is_match(&section.heading);
        if !focus && !contextual { continue; }
        let body = section.paragraphs.join(" ");
        for sentence in split_sentences(&body) {
            let (cue, confidence) = match detect_cue(&sentence) {
                Some(v) => v,
                None if focus => {
                    // Every substantive sentence in a limitations or
                    // future-work section is worth a look.
                    let c = if FUTURE_RE.is_match(&section.heading) { "future_work" } else { "limitation" };
                    (c, "medium")
                }
                None => continue,
            };
            push_candidate(&mut out, &mut seen, &paper.id, &section.heading, cue, &sentence, confidence, "");
        }
    }

    for finding in &paper.findings {
        if let Some(caveat) = &finding.caveat {
            push_candidate(&mut out, &mut seen, &paper.id, &format!("Finding {}", finding.id),
                "caveat", caveat, "high", &finding.id);
        }
    }

    for hypothesis in &paper.hypotheses {
        let unresolved = hypothesis.status == crate::models::HypothesisStatus::Open
            || hypothesis.evidence_ids.is_empty();
        if unresolved {
            push_candidate(&mut out, &mut seen, &paper.id, &format!("Hypothesis {}", hypothesis.id),
                "unresolved", &hypothesis.statement, "high", &hypothesis.id);
        }
    }

    for note in &paper.notes {
        if detect_cue(note).is_some() {
            push_candidate(&mut out, &mut seen, &paper.id, "Extraction note", "limitation", note, "medium", "");
        }
    }

    let rank = |c: &str| match c { "high" => 0, "medium" => 1, _ => 2 };
    out.sort_by_key(|c| rank(&c.confidence));
    out.truncate(40);
    out.into_iter().enumerate().map(|(i, c)| c.into_question(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cues_in_priority_order() {
        assert_eq!(detect_cue("What happens under adaptive attacks?"), Some(("question", "high")));
        assert_eq!(detect_cue("We leave this to future work."), Some(("future_work", "high")));
        assert_eq!(detect_cue("It remains unclear whether this transfers."), Some(("uncertainty", "high")));
        assert_eq!(detect_cue("This is a limitation of our evaluation."), Some(("limitation", "medium")));
        assert_eq!(detect_cue("We do not consider adaptive attackers."), Some(("negation", "medium")));
        assert_eq!(detect_cue("We assume a static threat model."), Some(("assumption", "low")));
        assert_eq!(detect_cue("The method uses four primitives."), None);
    }

    #[test]
    fn rewrites_common_declaratives_into_questions() {
        let q = suggest_question("We do not consider adaptive attackers.", "negation");
        assert_eq!(q, "What happens if we consider adaptive attackers?");
        let q = suggest_question("We assume a static threat model.", "assumption");
        assert_eq!(q, "What if the assumption that a static threat model does not hold?");
        let q = suggest_question("We only evaluate on synthetic data.", "limitation");
        assert_eq!(q, "Does the result hold without restricting to evaluate on synthetic data?");
    }

    #[test]
    fn generic_stems_always_end_with_a_question_mark() {
        for cue in ["future_work", "uncertainty", "limitation", "negation", "assumption", "caveat", "unresolved"] {
            let q = suggest_question("The result is reported only for one dataset.", cue);
            assert!(q.ends_with('?'), "{cue}: {q}");
            assert!(q.contains("dataset"), "{cue}: {q}");
        }
        assert_eq!(suggest_question("Does this scale?", "question"), "Does this scale?");
    }

    #[test]
    fn splits_sentences_on_terminal_punctuation() {
        let s = split_sentences("First point here. Second point there! Third point?");
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], "First point here.");
    }

    #[test]
    fn dedup_key_is_stable_and_case_insensitive() {
        assert_eq!(dedup_key("We do not consider Adaptive Attackers."), dedup_key("we do not consider adaptive attackers"));
    }

    #[test]
    fn extracts_candidates_from_curated_seed() {
        let paper = crate::seed::seed_paper();
        let found = extract(&paper);
        assert!(!found.is_empty(), "curated seed should yield open-question candidates");
        assert!(found.iter().all(|q| q.text.ends_with('?') && !q.context.is_empty()));
        assert!(found.iter().any(|q| q.cue == "future_work" || q.cue == "caveat"),
            "expected a future-work or caveat candidate, got {:?}", found.iter().map(|q| &q.cue).collect::<Vec<_>>());
        // The O_ext caveat is a real finding caveat and must survive extraction.
        assert!(found.iter().any(|q| q.context.to_lowercase().contains("empirical")),
            "expected the O_ext empirical/attack-specific caveat to be surfaced");
    }
}
