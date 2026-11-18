//! AI guardrails: source-anchored, checkable insights.
//!
//! Every discrete AI-written insight (a key insight or a caveat) is anchored
//! to a paper section and starts unverified. A one-click source check compares
//! the claim's key terms against that section's text and records what was and
//! was not found. This is a deterministic lexical guardrail — a signal for a
//! human reviewer, never a verdict on whether the claim is true. Human
//! verification and section corrections survive regeneration and re-import.

use crate::error::{AppError, AppResult};
use crate::models::*;
use chrono::Utc;
use serde::Serialize;
use std::collections::HashSet;

const MAX_TERMS: usize = 14;
const SUPPORT_THRESHOLD: f32 = 0.45;

fn stopwords() -> &'static HashSet<&'static str> {
    use std::sync::LazyLock;
    static STOP: LazyLock<HashSet<&'static str>> = LazyLock::new(|| [
        "the", "and", "for", "that", "with", "this", "from", "are", "was", "were", "has", "have",
        "had", "but", "not", "can", "could", "would", "should", "into", "about", "than", "then",
        "also", "more", "most", "some", "such", "only", "over", "under", "between", "through",
        "their", "there", "these", "those", "its", "our", "your", "his", "her", "they", "them",
        "been", "being", "does", "did", "doing", "each", "other", "which", "while", "where",
        "when", "what", "who", "how", "why", "all", "any", "both", "few", "own", "same", "too",
        "very", "just", "like", "make", "made", "makes", "show", "shows", "shown", "found",
        "find", "finds", "using", "used", "use", "uses", "based", "across", "however", "therefore",
        "results", "result", "study", "paper", "approach", "method", "methods", "work", "works",
        "model", "models", "propose", "proposed", "present", "presents", "demonstrate", "demonstrates",
        "suggest", "suggests", "indicate", "indicates", "important", "overall", "further", "given",
    ].into_iter().collect());
    &STOP
}

fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 4 && !w.chars().all(|c| c.is_ascii_digit()) && !stopwords().contains(w))
        .map(|w| w.to_string())
        .collect()
}

/// Distinct, meaningful claim terms, longest first, capped for a focused check.
fn terms(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tokens(text) {
        if out.iter().any(|x| x == &t) { continue; }
        out.push(t);
    }
    out.sort_by_key(|t| std::cmp::Reverse(t.len()));
    out.truncate(MAX_TERMS);
    out
}

/// Light plural/stem tolerance: "attacks" matches "attack", "permutations"
/// matches "permutation", without pulling in unrelated short words.
fn term_matches(term: &str, token: &str) -> bool {
    if term == token { return true; }
    if term.len() >= 6 && token.starts_with(term) { return true; }
    if token.len() >= 6 && term.starts_with(token) { return true; }
    false
}

/// Named text regions a claim can be anchored to.
fn sections_of(paper: &Paper) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for s in &paper.sections {
        if s.heading.trim().is_empty() { continue; }
        out.push((s.heading.trim().to_string(), s.paragraphs.join(" ")));
    }
    if !paper.findings.is_empty() {
        let text = paper.findings.iter().map(|f| {
            let mut t = format!("{} {}", f.claim, f.evidence);
            if let Some(c) = &f.caveat { t.push_str(&format!(" {c}")); }
            t
        }).collect::<Vec<_>>().join(" ");
        out.push(("Findings".into(), text));
    }
    if !paper.experiments.is_empty() {
        let text = paper.experiments.iter().map(|e| {
            let mut t = format!("{} {} {}", e.name, e.question, e.design);
            for r in &e.results { t.push_str(&format!(" {r}")); }
            t
        }).collect::<Vec<_>>().join(" ");
        out.push(("Experiments".into(), text));
    }
    out
}

fn whole_paper_text(paper: &Paper) -> String {
    let mut parts = vec![paper.title.clone(), paper.abstract_text.clone(), paper.summary.clone()];
    parts.extend(paper.sections.iter().flat_map(|s| s.paragraphs.iter().cloned()));
    parts.extend(paper.findings.iter().map(|f| format!("{} {}", f.claim, f.evidence)));
    parts.join(" ")
}

fn section_text<'a>(paper: &'a Paper, section: &str) -> String {
    if section.is_empty() { return whole_paper_text(paper); }
    sections_of(paper).into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(section))
        .map(|(_, text)| text)
        .unwrap_or_else(|| whole_paper_text(paper))
}

fn overlap(claim_terms: &[String], text: &str) -> usize {
    let hay: Vec<String> = tokens(text);
    claim_terms.iter().filter(|term| hay.iter().any(|t| term_matches(term, t))).count()
}

/// Pick the section that best supports the claim. A model-suggested name wins
/// when it matches a real section heading; otherwise the strongest lexical
/// overlap decides. Returns "" when nothing meaningful matches (whole paper).
fn best_section(paper: &Paper, text: &str, suggested: Option<&str>) -> String {
    let sections = sections_of(paper);
    if let Some(want) = suggested.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Some((name, _)) = sections.iter().find(|(name, _)| name.eq_ignore_ascii_case(want)) {
            return name.clone();
        }
    }
    let claim_terms = terms(text);
    if claim_terms.is_empty() { return String::new(); }
    let mut best = (String::new(), 0usize);
    for (name, body) in sections {
        let score = overlap(&claim_terms, &body);
        if score > best.1 { best = (name, score); }
    }
    best.0
}

/// Run the deterministic lexical support check for one claim.
fn check(paper: &Paper, text: &str, section: &str) -> (f32, Vec<String>, Vec<String>) {
    let claim_terms = terms(text);
    if claim_terms.is_empty() { return (0.0, Vec::new(), Vec::new()); }
    let body = section_text(paper, section);
    let hay: Vec<String> = tokens(&body);
    let mut matched = Vec::new();
    let mut missing = Vec::new();
    for term in &claim_terms {
        if hay.iter().any(|t| term_matches(term, t)) { matched.push(term.clone()); }
        else { missing.push(term.clone()); }
    }
    let support = matched.len() as f32 / claim_terms.len() as f32;
    (support, matched, missing)
}

/// The (key, category, text) triples that get anchors for one enhancement.
fn insight_items(enh: &PaperEnhancement) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for (i, text) in enh.key_insights.iter().enumerate() {
        out.push((format!("enhancement:key_insights:{i}"), "insight".into(), text.clone()));
    }
    for (i, text) in enh.caveats.iter().enumerate() {
        out.push((format!("enhancement:caveats:{i}"), "caveat".into(), text.clone()));
    }
    out
}

/// Create or refresh anchors for every current insight. Existing anchors keep
/// their verification state, section override and note; only the text snapshot
/// and support evidence are recomputed. Returns true if anything changed.
pub fn ensure_anchors(paper: &mut Paper, suggested: Option<&[String]>) -> bool {
    // Snapshot the insight list first so we never hold a mutable borrow of the
    // enhancement while running checks against the rest of the paper.
    let Some(items) = paper.enhancement.as_ref().map(insight_items) else { return false; };
    let old_anchors = paper.enhancement.as_ref().map(|e| e.insight_anchors.clone()).unwrap_or_default();
    let mut anchors: Vec<InsightAnchor> = Vec::new();
    let mut changed = false;

    for (idx, (key, category, text)) in items.iter().enumerate() {
        if let Some(existing) = old_anchors.iter().find(|a| a.key == *key) {
            let mut anchor = existing.clone();
            anchor.category = category.clone();
            if anchor.text != *text {
                anchor.text = text.clone();
                let (support, matched, missing) = check(paper, text, &anchor.section);
                anchor.support = support;
                anchor.matched_terms = matched;
                anchor.missing_terms = missing;
                changed = true;
            }
            anchors.push(anchor);
        } else {
            let suggestion = suggested.and_then(|s| s.get(idx)).map(|s| s.as_str());
            let section = best_section(paper, text, suggestion);
            let (support, matched, missing) = check(paper, text, &section);
            anchors.push(InsightAnchor {
                key: key.clone(),
                category: category.clone(),
                text: text.clone(),
                section,
                verification: InsightVerification::Unverified,
                human: false,
                checked_at: None,
                support,
                matched_terms: matched,
                missing_terms: missing,
                note: String::new(),
            });
            changed = true;
        }
    }

    // Drop anchors for insights that no longer exist after regeneration.
    if anchors.len() != old_anchors.len() { changed = true; }

    // Keep anchors in a stable order: key insights first, then caveats.
    let rank = |category: &str| if category == "insight" { 0 } else { 1 };
    anchors.sort_by(|a, b| {
        rank(&a.category).cmp(&rank(&b.category)).then_with(|| {
            let idx = |k: &str| k.rsplit(':').next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            idx(&a.key).cmp(&idx(&b.key))
        })
    });
    if let Some(e) = paper.enhancement.as_mut() { e.insight_anchors = anchors; }
    changed
}

/// Carry human/automated verification state onto freshly generated anchors,
/// matched by key. Used when an enhancement is regenerated.
pub fn carry_verification(new_anchors: &mut [InsightAnchor], old_anchors: &[InsightAnchor]) {
    for anchor in new_anchors.iter_mut() {
        if let Some(old) = old_anchors.iter().find(|a| a.key == anchor.key) {
            anchor.verification = old.verification;
            anchor.human = old.human;
            anchor.note = old.note.clone();
            anchor.checked_at = old.checked_at;
        }
    }
}

fn find_anchor_mut<'a>(paper: &'a mut Paper, key: &str) -> AppResult<&'a mut InsightAnchor> {
    let enh = paper.enhancement.as_mut()
        .ok_or_else(|| AppError::BadRequest("this paper has no AI enhancement".into()))?;
    enh.insight_anchors.iter_mut().find(|a| a.key == key)
        .ok_or_else(|| AppError::NotFound(format!("insight '{key}'")))
}

/// One-click "verify against paper": run the lexical source check and store the
/// evidence. Unless a human already set the state, the automated result sets
/// Supported / Contested from the support threshold.
pub fn verify_insight(paper: &mut Paper, key: &str) -> AppResult<InsightAnchor> {
    let (text, section, human) = {
        let anchor = find_anchor_mut(paper, key)?;
        (anchor.text.clone(), anchor.section.clone(), anchor.human)
    };
    let (support, matched, missing) = check(paper, &text, &section);
    let anchor = find_anchor_mut(paper, key)?;
    anchor.support = support;
    anchor.matched_terms = matched;
    anchor.missing_terms = missing;
    anchor.checked_at = Some(Utc::now());
    if !human {
        anchor.verification = if support >= SUPPORT_THRESHOLD {
            InsightVerification::Supported
        } else {
            InsightVerification::Contested
        };
    }
    Ok(anchor.clone())
}

/// Human override of the verification state, protected from future re-checks.
pub fn set_verification(
    paper: &mut Paper, key: &str, verification: InsightVerification, note: String,
) -> AppResult<InsightAnchor> {
    let anchor = find_anchor_mut(paper, key)?;
    anchor.verification = verification;
    anchor.note = note;
    anchor.human = true;
    Ok(anchor.clone())
}

/// Human correction of the cited section; re-runs the support check.
pub fn set_section(paper: &mut Paper, key: &str, section: String) -> AppResult<InsightAnchor> {
    let text = find_anchor_mut(paper, key)?.text.clone();
    let (support, matched, missing) = check(paper, &text, &section);
    let anchor = find_anchor_mut(paper, key)?;
    anchor.section = section;
    anchor.support = support;
    anchor.matched_terms = matched;
    anchor.missing_terms = missing;
    anchor.checked_at = Some(Utc::now());
    Ok(anchor.clone())
}

// -- Report ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct InsightView {
    pub key: String,
    pub category: String,
    pub text: String,
    pub section: String,
    pub verification: InsightVerification,
    pub human: bool,
    pub checked_at: Option<chrono::DateTime<Utc>>,
    pub support: f32,
    pub matched_terms: Vec<String>,
    pub missing_terms: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuardrailReport {
    pub paper_id: String,
    pub has_enhancement: bool,
    /// Section headings available for citation on this paper.
    pub sections: Vec<String>,
    pub insights: Vec<InsightView>,
    pub total: usize,
    pub verified: usize,
    pub contested: usize,
    pub unverified: usize,
}

fn view(a: &InsightAnchor) -> InsightView {
    InsightView {
        key: a.key.clone(), category: a.category.clone(), text: a.text.clone(),
        section: a.section.clone(), verification: a.verification, human: a.human,
        checked_at: a.checked_at, support: a.support,
        matched_terms: a.matched_terms.clone(), missing_terms: a.missing_terms.clone(),
        note: a.note.clone(),
    }
}

pub fn report(paper: &Paper) -> GuardrailReport {
    let mut sections: Vec<String> = sections_of(paper).into_iter().map(|(name, _)| name).collect();
    let mut seen = HashSet::new();
    sections.retain(|s| seen.insert(s.clone()));

    let anchors: Vec<InsightView> = paper.enhancement.as_ref()
        .map(|e| e.insight_anchors.iter().map(view).collect())
        .unwrap_or_default();
    let verified = anchors.iter().filter(|a| a.verification == InsightVerification::Supported).count();
    let contested = anchors.iter().filter(|a| a.verification == InsightVerification::Contested).count();

    GuardrailReport {
        paper_id: paper.id.clone(),
        has_enhancement: paper.enhancement.is_some(),
        sections,
        total: anchors.len(),
        verified,
        contested,
        unverified: anchors.iter().filter(|a| a.verification == InsightVerification::Unverified).count(),
        insights: anchors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PaperSection;

    fn paper_with_sections() -> Paper {
        let mut p = Paper::default();
        p.id = "p1".into();
        p.sections.push(PaperSection {
            heading: "Introduction".into(), level: 2, paragraphs: vec![
                "Obfuscation protects on-device models by scrambling their weights.".into(),
            ],
        });
        p.sections.push(PaperSection {
            heading: "Evaluation".into(), level: 2, paragraphs: vec![
                "Column permutation reduced the attack success rate substantially across defenses.".into(),
                "We measured a twenty percent drop on the evaluated pipeline.".into(),
            ],
        });
        p.findings.push(Finding {
            id: "find:p1:x".into(),
            claim: "Column permutation reduced attack success.".into(),
            evidence: "20% drop".into(),
            ..Default::default()
        });
        p.enhancement = Some(PaperEnhancement {
            key_insights: vec![
                "Column permutation reduced attack success across defenses.".into(),
                "The model achieves perfect universal security with no caveats.".into(),
            ],
            caveats: vec!["Only one defense pipeline was evaluated.".into()],
            ..Default::default()
        });
        p
    }

    #[test]
    fn anchors_cover_all_insights_and_start_unverified() {
        let mut p = paper_with_sections();
        assert!(ensure_anchors(&mut p, None));
        let report = report(&p);
        assert_eq!(report.total, 3);
        assert_eq!(report.unverified, 3);
        assert_eq!(report.insights[0].category, "insight");
        assert_eq!(report.insights[2].category, "caveat");
        assert_eq!(report.sections, vec!["Introduction", "Evaluation", "Findings"]);
    }

    #[test]
    fn model_suggested_section_is_used_when_valid() {
        let mut p = paper_with_sections();
        let suggested = ["Evaluation".to_string()];
        ensure_anchors(&mut p, Some(&suggested));
        assert_eq!(p.enhancement.as_ref().unwrap().insight_anchors[0].section, "Evaluation");
    }

    #[test]
    fn best_section_matches_by_token_overlap() {
        let mut p = paper_with_sections();
        ensure_anchors(&mut p, None);
        let anchors = &p.enhancement.as_ref().unwrap().insight_anchors;
        assert_eq!(anchors[0].section, "Evaluation");
    }

    #[test]
    fn verify_sets_supported_for_anchored_claim_and_contested_for_hallucination() {
        let mut p = paper_with_sections();
        ensure_anchors(&mut p, None);
        let keys: Vec<String> = p.enhancement.as_ref().unwrap().insight_anchors.iter()
            .map(|a| a.key.clone()).collect();
        let good = verify_insight(&mut p, &keys[0]).unwrap();
        assert_eq!(good.verification, InsightVerification::Supported);
        assert!(good.checked_at.is_some());
        assert!(!good.matched_terms.is_empty());

        let bad = verify_insight(&mut p, &keys[1]).unwrap();
        assert_eq!(bad.verification, InsightVerification::Contested);
    }

    #[test]
    fn human_verification_survives_recheck_and_carry_over() {
        let mut p = paper_with_sections();
        ensure_anchors(&mut p, None);
        let key = p.enhancement.as_ref().unwrap().insight_anchors[1].key.clone();
        set_verification(&mut p, &key, InsightVerification::Contested, "checked table 3".into()).unwrap();
        let after = verify_insight(&mut p, &key).unwrap();
        assert_eq!(after.verification, InsightVerification::Contested);
        assert!(after.human);
        assert_eq!(after.note, "checked table 3");

        let old = p.enhancement.as_ref().unwrap().insight_anchors.clone();
        let mut fresh = vec![InsightAnchor { key: key.clone(), ..Default::default() }];
        carry_verification(&mut fresh, &old);
        assert_eq!(fresh[0].verification, InsightVerification::Contested);
        assert_eq!(fresh[0].note, "checked table 3");
    }

    #[test]
    fn section_override_rechecks_support() {
        let mut p = paper_with_sections();
        ensure_anchors(&mut p, None);
        let keys: Vec<String> = p.enhancement.as_ref().unwrap().insight_anchors.iter()
            .map(|a| a.key.clone()).collect();
        let before = p.enhancement.as_ref().unwrap().insight_anchors[0].support;
        let after = set_section(&mut p, &keys[0], "Introduction".into()).unwrap();
        assert_ne!(before, after.support);
        assert_eq!(after.section, "Introduction");
    }

    #[test]
    fn anchors_refresh_when_insight_text_changes() {
        let mut p = paper_with_sections();
        ensure_anchors(&mut p, None);
        p.enhancement.as_mut().unwrap().key_insights[0] = "A totally different statement about gradients.".into();
        assert!(ensure_anchors(&mut p, None));
        let anchor = &p.enhancement.as_ref().unwrap().insight_anchors[0];
        assert_eq!(anchor.text, "A totally different statement about gradients.");
        assert_eq!(anchor.verification, InsightVerification::Unverified);
    }
}
