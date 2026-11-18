//! Self-contained, multi-format exports built from the deterministic extraction.

use crate::models::Paper;

fn html(s: &str) -> String {
    s.chars().flat_map(|c| match c {
        '&' => "&amp;".chars().collect::<Vec<_>>(),
        '<' => "&lt;".chars().collect::<Vec<_>>(),
        '>' => "&gt;".chars().collect::<Vec<_>>(),
        '"' => "&quot;".chars().collect::<Vec<_>>(),
        '\'' => "&#39;".chars().collect::<Vec<_>>(),
        _ => vec![c],
    }).collect()
}

fn csv(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn author_key(p: &Paper) -> String {
    let surname = p.authors.first()
        .and_then(|a| a.split_whitespace().last())
        .unwrap_or("paper")
        .chars().filter(|c| c.is_alphanumeric()).collect::<String>()
        .to_lowercase();
    let year = p.year.map(|y| y.to_string()).unwrap_or_else(|| "nd".into());
    let word = p.title.split_whitespace()
        .find(|w| w.len() > 4)
        .unwrap_or("paper")
        .chars().filter(|c| c.is_alphanumeric()).collect::<String>()
        .to_lowercase();
    format!("{surname}{year}{word}")
}

fn clip(s: &str, n: usize) -> &str {
    if s.len() <= n { return s; }
    let mut end = n;
    while !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

/// BibTeX entry for the paper, usable directly in a reference manager.
pub fn bibtex(p: &Paper) -> String {
    let authors = p.authors.join(" and ");
    let mut out = format!(
        "@article{{{key},\n  title   = {{{title}}},\n  author  = {{{authors}}},\n",
        key = author_key(p),
        title = p.title,
    );
    if let Some(y) = p.year { out.push_str(&format!("  year    = {{{y}}},\n")); }
    if let Some(v) = &p.venue { out.push_str(&format!("  journal = {{{v}}},\n")); }
    if p.url.contains("arxiv.org") {
        if let Some(id) = p.url.rsplit('/').next() {
            out.push_str(&format!("  eprint  = {{{id}}},\n  archivePrefix = {{arXiv}},\n"));
        }
    }
    out.push_str(&format!("  url     = {{{url}}},\n", url = p.url));
    out.push_str(&format!("  note    = {{Explained in Paper Explainer; source: {:?}}},\n", p.source));
    out.push_str("}\n");
    out
}

/// Findings as a flat CSV table.
pub fn findings_csv(p: &Paper) -> String {
    let mut out = String::from("id,claim,simplified,evidence,importance,experiment_id,caveat\n");
    for f in &p.findings {
        let row = [
            f.id.as_str(), f.claim.as_str(), f.simplified.as_str(), f.evidence.as_str(),
            &format!("{:?}", f.importance),
            f.experiment_id.as_deref().unwrap_or(""),
            f.caveat.as_deref().unwrap_or(""),
        ].map(csv).join(",");
        out.push_str(&row);
        out.push('\n');
    }
    out
}

/// Findings as a structured JSON document with paper context.
pub fn findings_json(p: &Paper) -> String {
    let value = serde_json::json!({
        "paper_id": p.id,
        "title": p.title,
        "url": p.url,
        "generated_by": "paper-explainer",
        "findings": p.findings,
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())
}

/// Slidev-compatible deck. Slides are separated by `---`; `Note:` lines are speaker notes.
pub fn deck(p: &Paper) -> String {
    let mut out = format!(
        "---\ntitle: {}\ntheme: default\nmermaid:\n  theme: neutral\n---\n\n",
        p.title.replace(':', " -")
    );
    out.push_str(&format!("# {}\n\n", p.title));
    out.push_str(&format!("{}\n\n*{}*\n\n", p.summary, p.authors.join(", ")));
    out.push_str("Note: Introduce the question in one sentence; do not read the summary.\n\n---\n\n");

    if let Some(h) = p.hypotheses.first() {
        out.push_str("## What was claimed\n\n");
        for h in &p.hypotheses {
            out.push_str(&format!("- **{}** ({:?}) — {}\n", h.id, h.status, clip(&h.statement, 240)));
        }
        out.push_str(&format!("\nNote: {} is the headline claim.\n\n---\n\n", h.id));
    }

    if let Some(e) = p.experiments.first() {
        out.push_str(&format!("## {}\n\n{}\n\n", e.name, clip(&e.design, 600)));
        if !e.setup.models.is_empty() { out.push_str(&format!("- Models: {}\n", e.setup.models.join(", "))); }
        if !e.setup.datasets.is_empty() { out.push_str(&format!("- Data: {}\n", e.setup.datasets.join(", "))); }
        if !e.setup.metrics.is_empty() { out.push_str(&format!("- Metrics: {}\n", e.setup.metrics.join(", "))); }
        out.push_str("\nNote: Explain the setup, then narrate the diagram left to right.\n\n---\n\n");
    }

    if let Some(d) = p.diagrams.iter().find(|d| !d.mermaid.trim().is_empty()) {
        out.push_str(&format!("## {}\n\n```mermaid\n{}\n```\n\n---\n\n", d.title, d.mermaid));
    }

    out.push_str("## What they found\n\n");
    for f in &p.findings {
        out.push_str(&format!("- **{}** — {} · _{}_\n", f.id, clip(&f.simplified, 260), clip(&f.evidence, 160)));
    }
    out.push_str("\nNote: Lead with the strongest result; keep the exact numbers visible.\n");

    if !p.notes.is_empty() || p.findings.iter().any(|f| f.caveat.is_some()) {
        out.push_str("\n---\n\n## What this does NOT show\n\n");
        for f in p.findings.iter().filter(|f| f.caveat.is_some()) {
            out.push_str(&format!("- {} — {}\n", f.id, f.caveat.as_deref().unwrap_or("")));
        }
        for n in &p.notes { out.push_str(&format!("- {}\n", n)); }
        out.push_str("\nNote: Do not skip this slide for a nontechnical audience.\n");
    }
    out
}

/// A designed, self-contained one-page brief. No external assets; prints cleanly to PDF.
pub fn brief_html(p: &Paper) -> String {
    let mut findings = String::new();
    for f in p.findings.iter().take(6) {
        findings.push_str(&format!(
            "<tr><td class='k'>{}</td><td><b>{}</b><br><span class='muted'>{}</span></td><td>{}</td></tr>",
            html(&f.id), html(&f.simplified), html(&f.claim), html(&f.evidence)
        ));
    }
    if findings.is_empty() { findings.push_str("<tr><td colspan='3' class='muted'>No findings extracted.</td></tr>"); }

    let mut caveats: Vec<String> = p.findings.iter().filter_map(|f| f.caveat.clone()).collect();
    if let Some(e) = &p.enhancement { caveats.extend(e.caveats.clone()); }
    caveats.extend(p.notes.clone());
    let caveats_html = if caveats.is_empty() {
        "<p class='muted'>No caveats recorded.</p>".to_string()
    } else {
        format!("<ul>{}</ul>", caveats.iter().take(5).map(|c| format!("<li>{}</li>", html(c))).collect::<String>())
    };

    let hypotheses = p.hypotheses.iter().take(4).map(|h| format!(
        "<li><b>{}</b> <span class='tag'>{:?}</span> — {}</li>",
        html(&h.id), h.status, html(&clip(&h.simplified, 220))
    )).collect::<String>();

    let tldr = p.enhancement.as_ref().map(|e| e.tldr.clone()).unwrap_or_else(|| p.summary.clone());
    let setup = p.experiments.first().map(|e| {
        let s = &e.setup;
        format!(
            "<p><b>{}</b> — {}</p><p class='muted'>{}</p>",
            html(&e.name), html(&clip(&e.design, 420)),
            html(&[
                if s.models.is_empty() { String::new() } else { format!("Models: {} · ", s.models.join(", ")) },
                if s.datasets.is_empty() { String::new() } else { format!("Data: {} · ", s.datasets.join(", ")) },
                if s.metrics.is_empty() { String::new() } else { format!("Metrics: {}", s.metrics.join(", ")) },
            ].concat())
        )
    }).unwrap_or_else(|| "<p class='muted'>No experiment setup extracted.</p>".into());

    format!(r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} — one-page brief</title>
<style>
:root{{--ink:#151a23;--muted:#5f6b7f;--line:#dfe3ec;--accent:#4f46e5;--soft:#eef2ff}}
*{{box-sizing:border-box}}body{{margin:0;background:#eef0f5;color:var(--ink);font:13px/1.5 -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}}
.page{{max-width:860px;margin:24px auto;padding:38px 44px;background:#fff;box-shadow:0 8px 30px rgba(20,25,40,.12);border-radius:8px}}
.eyebrow{{font-size:10px;letter-spacing:.14em;text-transform:uppercase;color:var(--accent);font-weight:700}}
h1{{font:600 30px/1.15 Georgia,serif;margin:8px 0 6px}}h2{{font:600 16px Georgia,serif;margin:22px 0 8px;padding-bottom:5px;border-bottom:1px solid var(--line)}}
.meta{{color:var(--muted);font-size:11px;margin-bottom:14px}}.tldr{{background:var(--soft);border-left:3px solid var(--accent);padding:12px 16px;border-radius:4px;font:16px/1.5 Georgia,serif}}
table{{width:100%;border-collapse:collapse}}td{{border-bottom:1px solid var(--line);padding:7px 8px;vertical-align:top}}td.k{{width:52px;font:11px ui-monospace,monospace;color:var(--accent)}}
.muted{{color:var(--muted)}}.tag{{font-size:10px;background:var(--soft);color:var(--accent);padding:1px 6px;border-radius:8px}}
ul{{margin:6px 0;padding-left:20px}}li{{margin-bottom:5px}}.foot{{margin-top:24px;padding-top:10px;border-top:1px solid var(--line);font-size:10px;color:var(--muted)}}
.btn{{display:inline-block;margin:0 0 14px;padding:8px 14px;background:var(--accent);color:#fff;border-radius:6px;text-decoration:none;font-weight:600}}
@media print{{body{{background:#fff}}.page{{margin:0;padding:0;max-width:none;box-shadow:none;border-radius:0}}.btn{{display:none}}@page{{size:A4;margin:14mm}}}}
</style></head><body>
<div class="page">
<a class="btn" href="javascript:window.print()">Print / Save as PDF</a>
<div class="eyebrow">One-page brief · Paper Explainer</div>
<h1>{title}</h1>
<div class="meta">{authors} · {venue} {year} · <a href="{url}">source ↗</a></div>
<div class="tldr">{tldr}</div>
<h2>The question</h2><p>{summary}</p>
<h2>What was claimed</h2><ul>{hypotheses}</ul>
<h2>Method</h2>{setup}
<h2>Findings</h2><table>{findings}</table>
<h2>What this does NOT show</h2>{caveats}
<div class="foot">Generated by Paper Explainer from deterministic extraction. Verify numbers against the source paper.</div>
</div></body></html>"#,
        title = html(&p.title),
        authors = html(&p.authors.join(", ")),
        venue = html(p.venue.as_deref().unwrap_or("arXiv")),
        year = p.year.map(|y| y.to_string()).unwrap_or_default(),
        url = html(&p.url),
        tldr = html(&clip(&tldr, 700)),
        summary = html(&clip(&p.summary, 900)),
        hypotheses = hypotheses,
        setup = setup,
        findings = findings,
        caveats = caveats_html,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Finding, Paper, PaperSource, PaperStatus};

    fn sample() -> Paper {
        let mut p = Paper {
            title: "A Study, With Things".into(),
            authors: vec!["Ada Lovelace".into(), "Grace Hopper".into()],
            year: Some(2024), url: "https://arxiv.org/html/2609.10117v2".into(),
            venue: Some("arXiv".into()), source: PaperSource::Curated, status: PaperStatus::Ready,
            ..Default::default()
        };
        p.findings.push(Finding {
            id: "F1".into(), claim: "It works, 12%".into(), simplified: "It works".into(),
            evidence: "p. 4, table 2".into(), importance: crate::models::Importance::High,
            ..Default::default()
        });
        p
    }

    #[test]
    fn bibtex_has_key_and_arxiv_eprint() {
        let b = bibtex(&sample());
        assert!(b.contains("@article{lovelace2024study,"));
        assert!(b.contains("eprint  = {2609.10117v2}"));
        assert!(b.contains("author  = {Ada Lovelace and Grace Hopper}"));
    }

    #[test]
    fn csv_quotes_commas() {
        let c = findings_csv(&sample());
        assert!(c.starts_with("id,claim,simplified,evidence,importance,experiment_id,caveat\n"));
        assert!(c.contains("\"It works, 12%\""));
    }

    #[test]
    fn json_contains_paper_context() {
        let j: serde_json::Value = serde_json::from_str(&findings_json(&sample())).unwrap();
        assert_eq!(j["paper_id"], "");
        assert_eq!(j["findings"][0]["id"], "F1");
    }

    #[test]
    fn deck_has_frontmatter_and_slide_breaks() {
        let d = deck(&sample());
        assert!(d.starts_with("---\ntitle:"));
        assert!(d.contains("\n---\n\n"));
        assert!(d.contains("Note:"));
    }

    #[test]
    fn brief_is_self_contained_and_escaped() {
        let h = brief_html(&sample());
        assert!(h.contains("<!doctype html>"));
        assert!(h.contains("@media print"));
        assert!(h.contains("A Study, With Things"));
        assert!(!h.contains("<script src"));
    }
}
