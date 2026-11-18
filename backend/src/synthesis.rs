//! Deterministic cross-paper synthesis: a related-work narrative, a comparison
//! matrix and a combined Mermaid diagram generated purely from the extraction.

use crate::models::{Importance, Paper, Synthesis, SynthesisRow};
use crate::util::{clean, make_id, truncate};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

/// Maximum number of papers a single synthesis may combine.
pub const MAX_PAPERS: usize = 8;

fn joined(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => format!("{}, and {}", items[..items.len() - 1].join(", "), items[items.len() - 1]),
    }
}

fn short_name(p: &Paper) -> String {
    let year = p.year.map(|y| y.to_string()).unwrap_or_else(|| "n.d.".into());
    format!("{} ({})", truncate(&p.title, 54), year)
}

fn category_label(id: &str) -> String {
    crate::categories::category_defs().into_iter()
        .find(|d| d.id == id).map(|d| d.label).unwrap_or_else(|| id.to_string())
}

fn shared_themes(papers: &[Paper], limit: usize) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, HashSet<usize>> = HashMap::new();
    for (i, p) in papers.iter().enumerate() {
        let mut tags: Vec<String> = p.tags.iter().map(|t| clean(t).to_lowercase()).collect();
        if !p.primary_category.is_empty() { tags.push(category_label(&p.primary_category).to_lowercase()); }
        for tag in tags {
            if tag.len() >= 3 { counts.entry(tag).or_default().insert(i); }
        }
    }
    let mut themes: Vec<(String, usize)> = counts.into_iter()
        .filter(|(_, set)| papers.len() == 1 || set.len() >= 2)
        .map(|(tag, set)| (tag, set.len()))
        .collect();
    themes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    themes.truncate(limit);
    themes
}

fn narrative(papers: &[Paper]) -> String {
    let mut out = Vec::new();
    out.push(format!("This synthesis draws together {} paper{}: {}.",
        papers.len(), if papers.len() == 1 { "" } else { "s" },
        joined(&papers.iter().map(short_name).collect::<Vec<_>>())));

    let themes = shared_themes(papers, 5);
    if !themes.is_empty() {
        let phrases = themes.iter()
            .map(|(t, c)| if papers.len() == 1 { t.clone() } else { format!("{} ({c} papers)", t) })
            .collect::<Vec<_>>();
        out.push(format!("{} across this set are {}.",
            if papers.len() == 1 { "The main themes are" } else { "Recurring themes are" },
            joined(&phrases)));
    }

    for p in papers {
        let claim = p.hypotheses.iter()
            .find(|h| h.status == crate::models::HypothesisStatus::Supported)
            .or_else(|| p.hypotheses.first())
            .map(|h| truncate(&h.statement, 150))
            .unwrap_or_else(|| truncate(&p.summary, 150));
        let method = p.experiments.first()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "no named experiment".into());
        let caveats = p.findings.iter().filter(|f| f.caveat.is_some()).count();
        out.push(format!("{} frames its central claim as “{}”. Its evaluation is anchored in {}, with {} finding{} recorded{}.",
            short_name(p), claim, method, p.findings.len(),
            if p.findings.len() == 1 { "" } else { "s" },
            if caveats > 0 { format!(" and {} caveated", caveats) } else { String::new() }));
    }

    let supported: usize = papers.iter().flat_map(|p| &p.hypotheses)
        .filter(|h| h.status == crate::models::HypothesisStatus::Supported).count();
    let open: usize = papers.iter().flat_map(|p| &p.hypotheses)
        .filter(|h| h.status == crate::models::HypothesisStatus::Open || h.evidence_ids.is_empty()).count();
    if supported > 0 || open > 0 {
        out.push(format!("Taken together, the set records {} supported claim{} and {} unresolved or unlinked claim{} — agreement here means the extractions agree, not that every result was independently replicated.",
            supported, if supported == 1 { "" } else { "s" },
            open, if open == 1 { "" } else { "s" }));
    }

    out.push("This narrative is assembled from deterministic extraction, so it inherits each paper’s scope conditions: a resistance or robustness result is evidence under the tested attack and conditions, not a universal guarantee. Verify anything consequential against the source papers.".into());
    out.join(" ")
}

fn key_finding(p: &Paper) -> String {
    p.findings.iter()
        .find(|f| f.importance == Importance::High)
        .or_else(|| p.findings.first())
        .map(|f| truncate(&f.claim, 180))
        .unwrap_or_else(|| "—".into())
}

fn status_mix(p: &Paper) -> String {
    let s = p.hypotheses.iter().filter(|h| h.status == crate::models::HypothesisStatus::Supported).count();
    let partial = p.hypotheses.iter().filter(|h| h.status == crate::models::HypothesisStatus::PartiallySupported).count();
    let rejected = p.hypotheses.iter().filter(|h| h.status == crate::models::HypothesisStatus::Rejected).count();
    let open = p.hypotheses.iter().filter(|h| h.status == crate::models::HypothesisStatus::Open).count();
    format!("{s} supported · {partial} partial · {rejected} rejected · {open} open")
}

fn row(dimension: &str, papers: &[Paper], value: fn(&Paper) -> String) -> SynthesisRow {
    SynthesisRow { dimension: dimension.into(), values: papers.iter().map(value).collect() }
}

fn comparison(papers: &[Paper]) -> Vec<SynthesisRow> {
    vec![
        row("Year", papers, |p| p.year.map(|y| y.to_string()).unwrap_or_else(|| "—".into())),
        row("Venue", papers, |p| p.venue.clone().unwrap_or_else(|| "—".into())),
        row("Category", papers, |p| if p.primary_category.is_empty() { "—".into() } else { category_label(&p.primary_category) }),
        row("Focus", papers, |p| truncate(&p.summary, 180)),
        row("Experiments", papers, |p| {
            let names: Vec<String> = p.experiments.iter().map(|e| e.name.clone()).collect();
            if names.is_empty() { "—".into() } else { truncate(&names.join("; "), 200) }
        }),
        row("Datasets", papers, |p| {
            let mut d: Vec<String> = p.experiments.iter().flat_map(|e| e.setup.datasets.clone()).collect();
            d.sort(); d.dedup();
            if d.is_empty() { "—".into() } else { truncate(&d.join(", "), 200) }
        }),
        row("Metrics", papers, |p| {
            let mut m: Vec<String> = p.experiments.iter().flat_map(|e| e.setup.metrics.clone()).collect();
            m.sort(); m.dedup();
            if m.is_empty() { "—".into() } else { truncate(&m.join(", "), 200) }
        }),
        row("Findings", papers, |p| p.findings.len().to_string()),
        row("Key finding", papers, key_finding),
        row("Hypotheses", papers, status_mix),
        row("Caveats", papers, |p| {
            let n = p.findings.iter().filter(|f| f.caveat.is_some()).count();
            format!("{n} recorded")
        }),
    ]
}

fn mermaid_label(s: &str) -> String {
    clean(s)
        .replace('"', "'")
        .replace('[', "(")
        .replace(']', ")")
        .replace('{', "(")
        .replace('}', ")")
        .replace('\n', " ")
}

fn diagram(papers: &[Paper]) -> String {
    let mut lines = vec!["flowchart LR".to_string()];
    for (i, p) in papers.iter().enumerate() {
        lines.push(format!("  P{i}[\"{}\"]", mermaid_label(&short_name(p))));
    }
    let themes = shared_themes(papers, 5);
    for (i, (theme, _)) in themes.iter().enumerate() {
        lines.push(format!("  T{i}(\"{}\")", mermaid_label(theme)));
    }
    for (i, p) in papers.iter().enumerate() {
        let mut tags: Vec<String> = p.tags.iter().map(|t| clean(t).to_lowercase()).collect();
        if !p.primary_category.is_empty() { tags.push(category_label(&p.primary_category).to_lowercase()); }
        for (ti, (theme, _)) in themes.iter().enumerate() {
            if tags.iter().any(|t| t == theme) { lines.push(format!("  P{i} --- T{ti}")); }
        }
    }
    if themes.is_empty() && papers.len() > 1 {
        for i in 1..papers.len() {
            lines.push(format!("  P{} --- P{i}", i - 1));
        }
    }
    lines.push("  classDef paper fill:#eef2ff,stroke:#6366f1,color:#312e81;".into());
    lines.push("  classDef theme fill:#fff7ed,stroke:#f97316,color:#7c2d12;".into());
    if !papers.is_empty() {
        lines.push(format!("  class {} paper;", (0..papers.len()).map(|i| format!("P{i}")).collect::<Vec<_>>().join(",")));
    }
    if !themes.is_empty() {
        lines.push(format!("  class {} theme;", (0..themes.len()).map(|i| format!("T{i}")).collect::<Vec<_>>().join(",")));
    }
    lines.join("\n")
}

/// Build a synthesis from an ordered, de-duplicated slice of papers.
pub fn build(papers: &[Paper], title: Option<String>, notes: Option<String>) -> Synthesis {
    let now = Utc::now();
    let paper_ids: Vec<String> = papers.iter().map(|p| p.id.clone()).collect();
    let default_title = match papers.len() {
        0 => "Untitled synthesis".into(),
        1 => format!("Synthesis: {}", truncate(&papers[0].title, 60)),
        n => format!("Synthesis of {n} papers"),
    };
    Synthesis {
        id: make_id("syn", &format!("{}-{}", paper_ids.join("-"), now.timestamp_millis())),
        title: title.map(|t| clean(&t)).filter(|t| !t.is_empty()).unwrap_or(default_title),
        paper_ids,
        paper_titles: papers.iter().map(|p| p.title.clone()).collect(),
        created_at: now,
        updated_at: now,
        narrative: narrative(papers),
        comparison: comparison(papers),
        diagram: diagram(papers),
        notes: notes.unwrap_or_default(),
    }
}

/// Regenerate the derived parts of a saved synthesis from its current paper set.
pub fn refresh(syn: &Synthesis, papers: &[Paper]) -> Synthesis {
    let mut next = build(papers, Some(syn.title.clone()), Some(syn.notes.clone()));
    next.id = syn.id.clone();
    next.created_at = syn.created_at;
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Paper> {
        let a = crate::seed::seed_paper();
        let mut b = crate::seed::seed_paper();
        b.id = "paper-b".into();
        b.title = "A Second Study of Obfuscation and Extraction".into();
        b.tags = vec!["security".into(), "obfuscation".into(), "benchmark".into()];
        vec![a, b]
    }

    #[test]
    fn builds_narrative_comparison_and_diagram() {
        let papers = sample();
        let syn = build(&papers, None, None);
        assert_eq!(syn.paper_ids.len(), 2);
        assert!(syn.narrative.contains("draws together 2 papers"));
        assert!(syn.narrative.contains("Recurring themes"));
        assert!(syn.narrative.contains("not a universal guarantee") || syn.narrative.contains("universal guarantee"));
        assert!(syn.comparison.iter().any(|r| r.dimension == "Datasets"));
        assert!(syn.comparison.iter().all(|r| r.values.len() == 2));
        assert!(syn.diagram.starts_with("flowchart LR"));
        assert!(syn.diagram.contains("P0 --- T") || syn.diagram.contains("P1 --- T"));
    }

    #[test]
    fn single_paper_synthesis_has_no_shared_theme_count() {
        let syn = build(&sample()[..1], None, None);
        assert_eq!(syn.paper_ids.len(), 1);
        assert!(syn.narrative.contains("1 paper:"));
        assert!(!syn.narrative.contains("(1 papers)"));
        assert!(syn.diagram.contains("P0"));
        assert!(syn.diagram.contains("T0"));
    }

    #[test]
    fn refresh_preserves_identity_and_user_notes() {
        let papers = sample();
        let syn = build(&papers, Some("My title".into()), Some("My notes".into()));
        let next = refresh(&syn, &papers);
        assert_eq!(next.id, syn.id);
        assert_eq!(next.created_at, syn.created_at);
        assert_eq!(next.notes, "My notes");
        assert_eq!(next.title, "My title");
    }
}
