//! Snapshots, field-level diffs and safe restore.
//!
//! Before a refresh or AI regeneration we copy the whole paper into a
//! `PaperSnapshot`. A generic walk over the two JSON documents produces a
//! field-level diff (dotted paths, array indices), so new fields need no special
//! handling. Restoring replaces extracted content but deliberately carries the
//! reader's manual edits — corrections, provenance overrides and human AI
//! verifications — onto the restored paper.

use crate::error::{AppResult, AppError};
use crate::models::*;
use crate::util::make_id;
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

const MAX_CHANGES: usize = 400;
const MAX_STRING: usize = 500;

/// Volatile bookkeeping that should not appear as a meaningful change.
fn ignored(path: &str) -> bool {
    path == "updated_at" || path.ends_with(".updated_at")
        || path == "added_at" || path.ends_with(".added_at")
}

fn clip_value(value: &Value) -> Value {
    match value {
        Value::String(s) if s.chars().count() > MAX_STRING => {
            let cut: String = s.chars().take(MAX_STRING - 1).collect();
            Value::String(format!("{cut}…"))
        }
        other => other.clone(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldChange {
    pub path: String,
    pub kind: ChangeKind,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperDiff {
    pub paper_id: String,
    pub snapshot_id: String,
    pub snapshot_label: String,
    pub snapshot_title: String,
    pub current_title: String,
    pub changes: Vec<FieldChange>,
    pub total: usize,
    pub truncated: bool,
    /// How many manual edits would be preserved if this snapshot were restored.
    pub manual_edits_preserved: usize,
}

fn push(out: &mut Vec<FieldChange>, path: String, kind: ChangeKind, before: &Value, after: &Value) {
    out.push(FieldChange {
        path, kind,
        before: clip_value(before),
        after: clip_value(after),
    });
}

/// Recursively walk two JSON values. Returns true if the change cap was hit.
fn walk(before: &Value, after: &Value, path: &str, out: &mut Vec<FieldChange>) -> bool {
    if ignored(path) { return false; }
    match (before, after) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let child = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => { if walk(x, y, &child, out) { return true; } }
                    (Some(x), None) => {
                        push(out, child, ChangeKind::Removed, x, &Value::Null);
                        if out.len() >= MAX_CHANGES { return true; }
                    }
                    (None, Some(y)) => {
                        push(out, child, ChangeKind::Added, &Value::Null, y);
                        if out.len() >= MAX_CHANGES { return true; }
                    }
                    _ => {}
                }
            }
            false
        }
        (Value::Array(a), Value::Array(b)) => {
            let max = a.len().max(b.len());
            for i in 0..max {
                let child = format!("{path}[{i}]");
                match (a.get(i), b.get(i)) {
                    (Some(x), Some(y)) => { if walk(x, y, &child, out) { return true; } }
                    (Some(x), None) => {
                        push(out, child, ChangeKind::Removed, x, &Value::Null);
                        if out.len() >= MAX_CHANGES { return true; }
                    }
                    (None, Some(y)) => {
                        push(out, child, ChangeKind::Added, &Value::Null, y);
                        if out.len() >= MAX_CHANGES { return true; }
                    }
                    _ => {}
                }
            }
            false
        }
        (x, y) if x != y => {
            push(out, path.to_string(), ChangeKind::Changed, x, y);
            out.len() >= MAX_CHANGES
        }
        _ => false,
    }
}

/// Count manual edits on a paper (corrections, overridden provenance entries,
/// human AI verifications) so the UI can state what restore will preserve.
pub fn manual_edit_count(paper: &Paper) -> usize {
    let provenance = paper.item_provenance.values().filter(|p| p.overridden).count();
    let anchors = paper.enhancement.as_ref()
        .map(|e| e.insight_anchors.iter().filter(|a| a.human).count())
        .unwrap_or(0);
    paper.corrections.len() + provenance + anchors
}

pub fn diff(snapshot: &PaperSnapshot, current: &Paper) -> AppResult<PaperDiff> {
    let current_json = serde_json::to_value(current)
        .map_err(|e| AppError::Internal(format!("could not serialise paper: {e}")))?;
    let mut changes = Vec::new();
    let truncated = walk(&snapshot.paper, &current_json, "", &mut changes);
    let total = changes.len();
    Ok(PaperDiff {
        paper_id: current.id.clone(),
        snapshot_id: snapshot.id.clone(),
        snapshot_label: snapshot.label.clone(),
        snapshot_title: snapshot.paper.get("title").and_then(Value::as_str)
            .unwrap_or(&snapshot.paper_title).to_string(),
        current_title: current.title.clone(),
        changes,
        total,
        truncated,
        manual_edits_preserved: manual_edit_count(current),
    })
}

/// Build a snapshot of a paper's current state.
pub fn create(paper: &Paper, reason: SnapshotReason, label: Option<String>) -> AppResult<PaperSnapshot> {
    let paper_json = serde_json::to_value(paper)
        .map_err(|e| AppError::Internal(format!("could not serialise paper: {e}")))?;
    let now = Utc::now();
    Ok(PaperSnapshot {
        id: make_id("snap", &format!("{}:{}", paper.id, now.timestamp_millis())),
        paper_id: paper.id.clone(),
        paper_title: paper.title.clone(),
        label: label.unwrap_or_else(|| reason.label().into()),
        reason,
        created_at: now,
        paper: paper_json,
    })
}

/// Restore a snapshot's extracted content while preserving the reader's manual
/// edits from the current paper. Corrections are re-applied to the restored
/// text, and provenance/guardrail state is carried across.
pub fn restore(snapshot: &PaperSnapshot, current: &Paper) -> AppResult<Paper> {
    let mut restored: Paper = serde_json::from_value(snapshot.paper.clone())
        .map_err(|e| AppError::Internal(format!("snapshot is not a valid paper: {e}")))?;
    if restored.id != current.id {
        return Err(AppError::BadRequest("snapshot belongs to a different paper".into()));
    }

    // Human edits always win over the older snapshot.
    restored.corrections = current.corrections.clone();
    for (key, prov) in &current.item_provenance {
        if prov.overridden {
            restored.item_provenance.insert(key.clone(), prov.clone());
        }
    }
    if let (Some(cur), Some(res)) = (&current.enhancement, &mut restored.enhancement) {
        let human: Vec<InsightAnchor> = cur.insight_anchors.iter()
            .filter(|a| a.human).cloned().collect();
        crate::guardrails::carry_verification(&mut res.insight_anchors, &human);
    }

    // Re-apply corrections to the restored text and refresh derived metadata so
    // the restored paper is immediately consistent.
    crate::corrections::apply_all(&mut restored);
    crate::provenance::backfill_paper(&mut restored);
    crate::guardrails::ensure_anchors(&mut restored, None);
    restored.updated_at = Utc::now();
    Ok(restored)
}

/// Keep the store bounded: at most `limit` newest snapshots per paper.
pub fn prune(snapshots: &mut Vec<PaperSnapshot>, paper_id: &str, limit: usize) {
    let mut indices: Vec<usize> = snapshots.iter().enumerate()
        .filter(|(_, s)| s.paper_id == paper_id)
        .map(|(i, _)| i).collect();
    if indices.len() <= limit { return; }
    indices.sort_by(|&a, &b| snapshots[b].created_at.cmp(&snapshots[a].created_at));
    let drop: HashSet<usize> = indices.into_iter().skip(limit).collect();
    let mut i = 0;
    snapshots.retain(|_| { let idx = i; i += 1; !drop.contains(&idx) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PaperSection;

    fn paper() -> Paper {
        let mut p = Paper::default();
        p.id = "p1".into();
        p.title = "Original title".into();
        p.summary = "An original summary.".into();
        p.sections.push(PaperSection {
            heading: "Intro".into(), level: 2, paragraphs: vec!["Some text about gradients.".into()],
        });
        p.enhancement = Some(PaperEnhancement {
            key_insights: vec!["A grounded insight about gradients.".into()],
            ..Default::default()
        });
        crate::provenance::backfill_paper(&mut p);
        crate::guardrails::ensure_anchors(&mut p, None);
        p
    }

    #[test]
    fn create_snapshot_captures_full_paper() {
        let p = paper();
        let snap = create(&p, SnapshotReason::Refresh, None).unwrap();
        assert_eq!(snap.paper_id, "p1");
        assert_eq!(snap.reason, SnapshotReason::Refresh);
        assert_eq!(snap.paper.get("title").and_then(Value::as_str), Some("Original title"));
    }

    #[test]
    fn diff_reports_changed_added_and_removed_fields() {
        let original = paper();
        let snap = create(&original, SnapshotReason::Refresh, None).unwrap();
        let mut changed = original.clone();
        changed.title = "New title".into();
        changed.tags.push("new-tag".into());
        changed.findings.clear();
        let diff = diff(&snap, &changed).unwrap();
        assert!(diff.changes.iter().any(|c| c.path == "title" && c.kind == ChangeKind::Changed));
        assert!(diff.changes.iter().any(|c| c.path == "tags[0]" && c.kind == ChangeKind::Added));
        assert!(!diff.changes.iter().any(|c| c.path.ends_with("updated_at")));
        assert_eq!(diff.total, diff.changes.len());
    }

    #[test]
    fn restore_protects_corrections_and_human_verification() {
        let mut original = paper();
        original.findings.push(Finding {
            id: "find:p1:f".into(), claim: "X improves Y by 12%.".into(), ..Default::default()
        });
        let snap = create(&original, SnapshotReason::Refresh, None).unwrap();

        let mut current = original.clone();
        current.title = "Drifted title".into();
        crate::provenance::backfill_paper(&mut current);
        crate::corrections::apply_correction(&mut current, "find:p1:f", "claim", "X improves Y by 21%.").unwrap();
        let insight_key = current.enhancement.as_ref().unwrap().insight_anchors[0].key.clone();
        crate::guardrails::set_verification(&mut current, &insight_key,
            InsightVerification::Contested, "checked by hand".into()).unwrap();

        let restored = restore(&snap, &current).unwrap();
        assert_eq!(restored.title, "Original title"); // extracted content reverted
        assert_eq!(restored.findings[0].claim, "X improves Y by 21%."); // correction survived
        assert_eq!(restored.corrections.len(), 1);
        let anchor = restored.enhancement.as_ref().unwrap().insight_anchors.iter()
            .find(|a| a.key == insight_key).unwrap();
        assert_eq!(anchor.verification, InsightVerification::Contested);
        assert_eq!(anchor.note, "checked by hand");
    }

    #[test]
    fn prune_keeps_only_newest_snapshots_for_a_paper() {
        let p = paper();
        let mut snaps: Vec<PaperSnapshot> = (0..5).map(|i| {
            let mut s = create(&p, SnapshotReason::Manual, None).unwrap();
            s.created_at = Utc::now() + chrono::Duration::seconds(i);
            s
        }).collect();
        let mut other = create(&p, SnapshotReason::Manual, None).unwrap();
        other.paper_id = "other".into();
        snaps.push(other);
        prune(&mut snaps, "p1", 2);
        assert_eq!(snaps.iter().filter(|s| s.paper_id == "p1").count(), 2);
        assert_eq!(snaps.iter().filter(|s| s.paper_id == "other").count(), 1);
    }
}
