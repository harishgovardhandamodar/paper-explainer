//! Inline field corrections and the extraction-quality dashboard.
//!
//! Users fix extracted wording in place. Each edit is stored as an audit
//! record, flips that item's provenance to `manual`, and is re-applied after a
//! re-import so a refresh cannot silently restore a value the reader already
//! corrected. The dashboard joins those corrections to the extraction rule
//! that produced each item, so weak rules become visible rather than hiding
//! behind an aggregate accuracy number.

use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::provenance::item_type;
use crate::util::make_id;
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;

/// Fields a reader may edit, with UI labels. Text fields only — structure such
/// as ids, links and Mermaid source is deliberately read-only here.
pub fn editable_fields(item_key: &str) -> Vec<(&'static str, &'static str)> {
    if item_key == "summary" {
        return vec![("summary", "Plain-language summary")];
    }
    if let Some(rest) = item_key.strip_prefix("enhancement:") {
        return match rest {
            "tldr" => vec![("tldr", "One-sentence summary")],
            "explanation" => vec![("explanation", "Explanation")],
            "how_it_works" => vec![("how_it_works", "How it works")],
            other if other.starts_with("key_insights:") => vec![("value", "Insight")],
            other if other.starts_with("caveats:") => vec![("value", "Caveat")],
            _ => Vec::new(),
        };
    }
    match item_type(item_key) {
        "hypothesis" => vec![("statement", "Claim as stated"), ("simplified", "Plain language")],
        "finding" => vec![
            ("claim", "Claim"),
            ("simplified", "Plain language"),
            ("evidence", "Evidence / numbers"),
            ("caveat", "Caveat (empty = none)"),
        ],
        "experiment" => vec![("name", "Name"), ("question", "Question"), ("design", "Design")],
        "concept" => vec![("name", "Term"), ("definition", "Definition")],
        "diagram" => vec![("title", "Title"), ("caption", "Caption")],
        "workflow" => vec![("title", "Title"), ("description", "Description")],
        _ => Vec::new(),
    }
}

fn field_mut<'a>(paper: &'a mut Paper, item_key: &str, field: &str) -> AppResult<&'a mut String> {
    if item_key == "summary" {
        return if field == "summary" { Ok(&mut paper.summary) }
            else { Err(AppError::BadRequest(format!("summary has no field '{field}'"))) };
    }
    if let Some(rest) = item_key.strip_prefix("enhancement:") {
        let enh = paper.enhancement.as_mut()
            .ok_or_else(|| AppError::NotFound("this paper has no AI enhancement".into()))?;
        return match rest {
            "tldr" if field == "tldr" => Ok(&mut enh.tldr),
            "explanation" if field == "explanation" => Ok(&mut enh.explanation),
            "how_it_works" if field == "how_it_works" => Ok(&mut enh.how_it_works),
            other if field == "value" => {
                let (list, idx) = other.rsplit_once(':')
                    .ok_or_else(|| AppError::BadRequest(format!("unknown item '{item_key}'")))?;
                let i = idx.parse::<usize>()
                    .map_err(|_| AppError::BadRequest(format!("invalid item key '{item_key}'")))?;
                match list {
                    "key_insights" => enh.key_insights.get_mut(i)
                        .ok_or_else(|| AppError::NotFound(format!("insight {i}"))),
                    "caveats" => enh.caveats.get_mut(i)
                        .ok_or_else(|| AppError::NotFound(format!("caveat {i}"))),
                    _ => Err(AppError::BadRequest(format!("unknown item '{item_key}'"))),
                }
            }
            _ => Err(AppError::BadRequest(format!("{item_key} has no field '{field}'"))),
        };
    }
    if let Some(h) = paper.hypotheses.iter_mut().find(|h| h.id == item_key) {
        return match field {
            "statement" => Ok(&mut h.statement),
            "simplified" => Ok(&mut h.simplified),
            _ => Err(AppError::BadRequest(format!("hypothesis has no field '{field}'"))),
        };
    }
    if let Some(f) = paper.findings.iter_mut().find(|f| f.id == item_key) {
        return match field {
            "claim" => Ok(&mut f.claim),
            "simplified" => Ok(&mut f.simplified),
            "evidence" => Ok(&mut f.evidence),
            _ => Err(AppError::BadRequest(format!("finding has no field '{field}'"))),
        };
    }
    if let Some(e) = paper.experiments.iter_mut().find(|e| e.id == item_key) {
        return match field {
            "name" => Ok(&mut e.name),
            "question" => Ok(&mut e.question),
            "design" => Ok(&mut e.design),
            _ => Err(AppError::BadRequest(format!("experiment has no field '{field}'"))),
        };
    }
    if let Some(c) = paper.concepts.iter_mut().find(|c| c.id == item_key) {
        return match field {
            "name" => Ok(&mut c.name),
            "definition" => Ok(&mut c.definition),
            _ => Err(AppError::BadRequest(format!("concept has no field '{field}'"))),
        };
    }
    if let Some(d) = paper.diagrams.iter_mut().find(|d| d.id == item_key) {
        return match field {
            "title" => Ok(&mut d.title),
            "caption" => Ok(&mut d.caption),
            _ => Err(AppError::BadRequest(format!("diagram has no field '{field}'"))),
        };
    }
    if let Some(w) = paper.workflows.iter_mut().find(|w| w.id == item_key) {
        return match field {
            "title" => Ok(&mut w.title),
            "description" => Ok(&mut w.description),
            _ => Err(AppError::BadRequest(format!("workflow has no field '{field}'"))),
        };
    }
    Err(AppError::NotFound(format!("item '{item_key}'")))
}

/// A finding's caveat is optional, so it needs its own accessor.
fn caveat_mut<'a>(paper: &'a mut Paper, item_key: &str, field: &str) -> AppResult<Option<&'a mut Option<String>>> {
    if field == "caveat" && item_type(item_key) == "finding" {
        let f = paper.findings.iter_mut().find(|f| f.id == item_key)
            .ok_or_else(|| AppError::NotFound(format!("item '{item_key}'")))?;
        Ok(Some(&mut f.caveat))
    } else {
        Ok(None)
    }
}

pub fn get_field(paper: &Paper, item_key: &str, field: &str) -> AppResult<String> {
    if let Some(_) = editable_fields(item_key).into_iter().find(|(f, _)| *f == field) {
        // Work through a temporary clone so borrowed and optional fields share
        // one code path without aliasing the live paper.
        let mut copy = paper.clone();
        if field == "caveat" && item_type(item_key) == "finding" {
            let f = copy.findings.iter().find(|f| f.id == item_key)
                .ok_or_else(|| AppError::NotFound(format!("item '{item_key}'")))?;
            return Ok(f.caveat.clone().unwrap_or_default());
        }
        return Ok(field_mut(&mut copy, item_key, field)?.clone());
    }
    Err(AppError::BadRequest(format!("'{field}' is not editable on {item_key}")))
}

/// Apply one edit, record it and mark the item as manually changed.
pub fn apply_correction(
    paper: &mut Paper, item_key: &str, field: &str, corrected: &str,
) -> AppResult<FieldCorrection> {
    if corrected.chars().count() > 8000 {
        return Err(AppError::BadRequest("corrected text is too long (8000 characters max)".into()));
    }
    let original = get_field(paper, item_key, field)?;
    let now = Utc::now();
    let id = make_id("corr", &format!("{}:{}:{}", paper.id, item_key, field));
    let correction = FieldCorrection {
        id,
        paper_id: paper.id.clone(),
        item_key: item_key.into(),
        field: field.into(),
        original: original.clone(),
        corrected: corrected.into(),
        created_at: now,
        updated_at: now,
    };

    // Write the value through the same whitelist used for reading.
    if let Some(slot) = caveat_mut(paper, item_key, field)? {
        *slot = if corrected.trim().is_empty() { None } else { Some(corrected.into()) };
    } else {
        *field_mut(paper, item_key, field)? = corrected.into();
    }

    let prov = paper.item_provenance.entry(item_key.into()).or_insert_with(Provenance::default);
    if prov.rule.is_empty() { prov.rule = "manual_correction".into(); }
    prov.kind = ProvenanceKind::Manual;
    prov.overridden = true;
    prov.updated_at = now;

    if let Some(existing) = paper.corrections.iter_mut().find(|c| c.id == correction.id) {
        let mut saved = correction;
        saved.created_at = existing.created_at;
        *existing = saved.clone();
        Ok(saved)
    } else {
        paper.corrections.push(correction.clone());
        Ok(correction)
    }
}

/// Restore an item to its pre-correction wording and drop the audit record.
pub fn revert_correction(paper: &mut Paper, correction_id: &str) -> AppResult<()> {
    let correction = paper.corrections.iter().find(|c| c.id == correction_id).cloned()
        .ok_or_else(|| AppError::NotFound(format!("correction '{correction_id}'")))?;
    if let Some(slot) = caveat_mut(paper, &correction.item_key, &correction.field)? {
        *slot = if correction.original.trim().is_empty() { None } else { Some(correction.original.clone()) };
    } else {
        *field_mut(paper, &correction.item_key, &correction.field)? = correction.original.clone();
    }
    paper.corrections.retain(|c| c.id != correction_id);

    if !paper.corrections.iter().any(|c| c.item_key == correction.item_key) {
        if let Some(prov) = paper.item_provenance.get_mut(&correction.item_key) {
            prov.kind = if prov.verified_at.is_some() { ProvenanceKind::Verified } else { prov.origin };
            prov.overridden = false;
            prov.updated_at = Utc::now();
        }
    }
    Ok(())
}

/// Re-apply stored corrections after a re-import replaced extracted fields.
pub fn apply_all(paper: &mut Paper) {
    let corrections = paper.corrections.clone();
    for c in corrections {
        let result = if c.field == "caveat" && item_type(&c.item_key) == "finding" {
            caveat_mut(paper, &c.item_key, &c.field).map(|slot| {
                if let Some(slot) = slot {
                    *slot = if c.corrected.trim().is_empty() { None } else { Some(c.corrected.clone()) };
                }
            })
        } else {
            field_mut(paper, &c.item_key, &c.field).map(|slot| *slot = c.corrected.clone())
        };
        let _ = result; // items removed by a new extraction are simply skipped
    }
}

/// One item the reader may correct, with its editable fields and current
/// values. Keeps the UI in sync with the backend whitelist.
#[derive(Debug, Clone, Serialize)]
pub struct EditableField {
    pub name: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditableItem {
    pub key: String,
    pub label: String,
    pub item_type: String,
    pub badge: ProvenanceKind,
    pub verified: bool,
    pub fields: Vec<EditableField>,
    pub current: HashMap<String, String>,
}

pub fn editable_items(paper: &Paper) -> Vec<EditableItem> {
    let mut out = Vec::new();
    for (key, prov) in &paper.item_provenance {
        let fields = editable_fields(key);
        if fields.is_empty() { continue; }
        let field_defs: Vec<EditableField> = fields.iter().map(|(name, label)| EditableField {
            name: (*name).into(), label: (*label).into(),
        }).collect();
        let current: HashMap<String, String> = fields.iter()
            .filter_map(|(name, _)| get_field(paper, key, name).ok().map(|v| (name.to_string(), v)))
            .collect();
        out.push(EditableItem {
            key: key.clone(),
            label: crate::provenance::label_for(paper, key),
            item_type: item_type(key).into(),
            badge: prov.kind,
            verified: prov.verified_at.is_some(),
            fields: field_defs,
            current,
        });
    }
    out.sort_by(|a, b| a.item_type.cmp(&b.item_type).then(a.label.cmp(&b.label)));
    out
}

// -- Extraction-quality dashboard --------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CorrectionView {
    pub id: String,
    pub paper_id: String,
    pub paper_title: String,
    pub item_key: String,
    pub item_type: String,
    pub field: String,
    pub original: String,
    pub corrected: String,
    pub rule: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleQuality {
    pub rule: String,
    pub items: usize,
    pub corrections: usize,
    pub verified: usize,
    pub low_confidence: usize,
    pub average_confidence: f32,
    pub correction_rate: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct QualityReport {
    pub items: usize,
    pub corrections: usize,
    pub verified: usize,
    pub low_confidence: usize,
    pub average_confidence: f32,
    pub rules: Vec<RuleQuality>,
    pub recent: Vec<CorrectionView>,
}

#[derive(Default)]
struct Accum {
    items: usize,
    corrections: usize,
    verified: usize,
    low: usize,
    conf_sum: f32,
}

pub fn quality_report(papers: &[Paper]) -> QualityReport {
    let mut by_rule: HashMap<String, Accum> = HashMap::new();
    let mut recent = Vec::new();

    for paper in papers {
        for (key, prov) in &paper.item_provenance {
            let acc = by_rule.entry(prov.rule.clone()).or_default();
            acc.items += 1;
            acc.conf_sum += prov.confidence;
            if prov.verified_at.is_some() { acc.verified += 1; }
            if prov.confidence < 0.5 { acc.low += 1; }
            acc.corrections += paper.corrections.iter().filter(|c| c.item_key == *key).count();
        }
        for c in &paper.corrections {
            recent.push(CorrectionView {
                id: c.id.clone(),
                paper_id: paper.id.clone(),
                paper_title: paper.title.clone(),
                item_key: c.item_key.clone(),
                item_type: item_type(&c.item_key).into(),
                field: c.field.clone(),
                original: c.original.clone(),
                corrected: c.corrected.clone(),
                rule: paper.item_provenance.get(&c.item_key)
                    .map(|p| p.rule.clone()).unwrap_or_default(),
                created_at: c.created_at,
                updated_at: c.updated_at,
            });
        }
    }

    let mut rules: Vec<RuleQuality> = by_rule.into_iter().map(|(rule, a)| RuleQuality {
        rule,
        items: a.items,
        corrections: a.corrections,
        verified: a.verified,
        low_confidence: a.low,
        average_confidence: if a.items == 0 { 0.0 } else { a.conf_sum / a.items as f32 },
        correction_rate: if a.items == 0 { 0.0 } else { a.corrections as f32 / a.items as f32 },
    }).collect();
    rules.sort_by(|a, b| b.corrections.cmp(&a.corrections)
        .then(b.low_confidence.cmp(&a.low_confidence))
        .then(a.rule.cmp(&b.rule)));
    recent.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    recent.truncate(40);

    let total_items: usize = rules.iter().map(|r| r.items).sum();
    let total_corrections: usize = rules.iter().map(|r| r.corrections).sum();
    QualityReport {
        items: total_items,
        corrections: total_corrections,
        verified: rules.iter().map(|r| r.verified).sum(),
        low_confidence: rules.iter().map(|r| r.low_confidence).sum(),
        average_confidence: if total_items == 0 { 0.0 } else {
            rules.iter().map(|r| r.average_confidence * r.items as f32).sum::<f32>() / total_items as f32
        },
        rules,
        recent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Finding, Hypothesis};
    use crate::provenance::backfill_paper;

    fn paper_with_items() -> Paper {
        let mut p = Paper::default();
        p.id = "p1".into();
        p.title = "A Paper".into();
        p.source = PaperSource::Heuristic;
        p.abstract_text = "We study something.".into();
        p.hypotheses.push(Hypothesis {
            id: "hyp:p1:x".into(), statement: "X helps Y.".into(), simplified: "X is useful.".into(),
            ..Default::default()
        });
        p.findings.push(Finding {
            id: "find:p1:result".into(), claim: "X improves Y by 12%.".into(),
            simplified: "Better results.".into(), evidence: "12%".into(),
            ..Default::default()
        });
        backfill_paper(&mut p);
        p
    }

    #[test]
    fn correction_edits_field_records_audit_and_marks_manual() {
        let mut p = paper_with_items();
        let corr = apply_correction(&mut p, "find:p1:result", "claim", "X improves Y by 21%.").unwrap();
        assert_eq!(corr.original, "X improves Y by 12%.");
        assert_eq!(p.findings[0].claim, "X improves Y by 21%.");
        assert_eq!(p.item_provenance["find:p1:result"].kind, ProvenanceKind::Manual);
        assert!(p.item_provenance["find:p1:result"].overridden);
        assert_eq!(p.corrections.len(), 1);
    }

    #[test]
    fn reapplying_correction_survives_reimport() {
        let mut p = paper_with_items();
        apply_correction(&mut p, "hyp:p1:x", "statement", "X definitely helps Y.").unwrap();
        let mut fresh = paper_with_items();
        fresh.corrections = p.corrections.clone();
        apply_all(&mut fresh);
        assert_eq!(fresh.hypotheses[0].statement, "X definitely helps Y.");
    }

    #[test]
    fn caveat_is_optional_and_revert_restores_it() {
        let mut p = paper_with_items();
        apply_correction(&mut p, "find:p1:result", "caveat", "Only on one dataset").unwrap();
        assert_eq!(p.findings[0].caveat.as_deref(), Some("Only on one dataset"));
        let id = p.corrections[0].id.clone();
        revert_correction(&mut p, &id).unwrap();
        assert_eq!(p.findings[0].caveat, None);
        assert!(p.corrections.is_empty());
    }

    #[test]
    fn quality_report_groups_corrections_by_rule() {
        let mut p = paper_with_items();
        apply_correction(&mut p, "find:p1:result", "claim", "Updated claim.").unwrap();
        let report = quality_report(&[p]);
        assert_eq!(report.corrections, 1);
        assert!(report.rules.iter().any(|r| r.rule == "finding_numbers" && r.corrections == 1));
        assert_eq!(report.recent.len(), 1);
        assert_eq!(report.recent[0].field, "claim");
    }

    #[test]
    fn whitelist_rejects_uneditable_fields() {
        let mut p = paper_with_items();
        assert!(apply_correction(&mut p, "find:p1:result", "id", "hacked").is_err());
        assert!(apply_correction(&mut p, "find:p1:result", "claim", "ok").is_ok());
    }
}
