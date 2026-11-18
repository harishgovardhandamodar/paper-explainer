//! Citation parsing, resolution and (optional) Crossref enrichment.
//!
//! Bibliography entries are mined from the references section at ingest time
//! and stored on each `Paper`. Resolution against the local collection is
//! deterministic and re-run whenever the graph is rebuilt, so a reference
//! becomes a `cites` edge as soon as the cited paper is added. Crossref is a
//! best-effort enrichment only — nothing here is required for the core graph.

use crate::error::{AppError, AppResult};
use crate::identifiers;
use crate::models::*;
use crate::util::make_id;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;

static DOI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b10\.\d{4,9}/[^\s,;)}\]]+").unwrap());
static ARXIV_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\barxiv\s*:\s*)?((?:\d{4}\.\d{4,5}|[a-z][a-z-]*/\d{7})(?:v\d+)?)").unwrap()
});
static YEAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap());
static WORD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z][A-Za-z'-]{2,}").unwrap());

/// Parse raw bibliography lines into structured, deduplicated references.
pub fn parse_references(lines: &[String], paper_id: &str) -> Vec<PaperReference> {
    let mut out: Vec<PaperReference> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let raw = line.trim();
        if raw.chars().count() < 15 { continue; }
        let mut r = PaperReference {
            id: format!("ref:{}:{}", paper_id, make_id("", raw).trim_start_matches([':', '-'])),
            raw: raw.into(),
            ..Default::default()
        };
        r.doi = DOI_RE.find(raw).map(|m| {
            m.as_str().trim_end_matches(['.', ',', ';']).to_string()
        }).or_else(|| identifiers::normalize_doi(raw));
        if let Some(caps) = ARXIV_RE.captures(raw) {
            let id = caps.get(1).unwrap().as_str();
            if let Some(resolved) = identifiers::parse_arxiv(id) {
                r.arxiv_id = resolved.bare_id;
            }
        }
        r.year = YEAR_RE.captures(raw)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .filter(|y| *y >= 1900 && *y <= 2100);
        let (title, authors) = title_and_authors(raw);
        r.title = title;
        r.authors = authors;

        let dedup_key = r.arxiv_id.clone()
            .or_else(|| r.doi.clone().map(|d| d.to_lowercase()))
            .or_else(|| r.title.clone().map(|t| normalize_title(&t)));
        if	let Some(key) = dedup_key {
            if out.iter().any(|x| {
                x.arxiv_id.clone().or_else(|| x.doi.clone().map(|d| d.to_lowercase()))
                    .or_else(|| x.title.clone().map(|t| normalize_title(&t))) == Some(key.clone())
            }) { continue; }
        }
        let _ = i;
        out.push(r);
    }
    out.truncate(120);
    out
}

/// Heuristic split of a bibliography line into a probable title and author
/// list. Bibliography formats vary wildly; this favours recall over precision
/// and always keeps the raw line for the reader to verify.
fn title_and_authors(raw: &str) -> (Option<String>, Vec<String>) {
    let clean = raw.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '[' || c == ']').trim();
    let segments: Vec<&str> = clean.split(". ")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut title_idx = None;
    for (i, seg) in segments.iter().enumerate() {
        let words = WORD_RE.find_iter(seg).count();
        let looks_authors = seg.contains(" and ") || seg.contains(',') && words <= 8;
        if words >= 4 && !looks_authors && !seg.to_lowercase().starts_with("in ") {
            title_idx = Some(i);
            break;
        }
    }

    let title = title_idx.and_then(|i| {
        let t = segments[i].trim_end_matches('.').trim();
        if t.chars().count() >= 8 { Some(t.to_string()) } else { None }
    });
    let authors = match title_idx {
        Some(0) => Vec::new(),
        Some(i) => segments[..i].iter()
            .flat_map(|s| s.split(" and "))
            .flat_map(|s| s.split(','))
            .map(|s| s.trim().trim_end_matches('.').to_string())
            .filter(|s| !s.is_empty() && s.chars().count() < 60 && WORD_RE.is_match(s))
            .collect(),
        None => Vec::new(),
    };
    (title, authors)
}

fn normalize_title(s: &str) -> String {
    WORD_RE.find_iter(&s.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Re-resolve every reference against the current collection. Called from the
/// store's graph rebuild so newly added papers automatically satisfy older
/// references. Pure with respect to everything except `resolved_paper_id`.
pub fn resolve_all(papers: &mut [Paper]) {
    let by_arxiv: HashMap<String, String> = papers.iter()
        .filter_map(|p| p.arxiv_id.as_ref().map(|a| (a.clone(), p.id.clone())))
        .collect();
    let by_doi: HashMap<String, String> = papers.iter()
        .filter_map(|p| p.doi.as_ref().map(|d| (d.to_lowercase(), p.id.clone())))
        .collect();
    let by_title: HashMap<String, String> = papers.iter()
        .filter_map(|p| {
            let key = normalize_title(&p.title);
            if key.is_empty() { None } else { Some((key, p.id.clone())) }
        })
        .collect();

    for p in papers.iter_mut() {
        for r in &mut p.references {
            r.resolved_paper_id = r.arxiv_id.as_ref().and_then(|a| by_arxiv.get(a).cloned())
                .or_else(|| r.doi.as_ref().and_then(|d| by_doi.get(&d.to_lowercase()).cloned()))
                .or_else(|| r.title.as_ref().and_then(|t| by_title.get(&normalize_title(t)).cloned()))
                .filter(|id| id != &p.id);
        }
    }
}

/// Per-paper citation summary for the dedicated view.
#[derive(Debug, Clone, Serialize)]
pub struct CitationEntry {
    pub paper_id: String,
    pub title: String,
    pub cites: Vec<String>,
    pub cited_by: Vec<String>,
    pub unresolved: usize,
}

pub fn summary(papers: &[Paper]) -> Vec<CitationEntry> {
    let title_by_id: HashMap<&str, &str> = papers.iter()
        .map(|p| (p.id.as_str(), p.title.as_str())).collect();
    let mut cited_by: HashMap<&str, Vec<String>> = HashMap::new();
    let mut entries = Vec::new();

    for p in papers {
        let mut cites = Vec::new();
        let mut unresolved = 0;
        for r in &p.references {
            if let Some(id) = &r.resolved_paper_id {
                cites.push(id.clone());
                cited_by.entry(id.as_str()).or_default().push(p.id.clone());
            } else {
                unresolved += 1;
            }
        }
        entries.push(CitationEntry {
            paper_id: p.id.clone(),
            title: p.title.clone(),
            cites,
            cited_by: Vec::new(),
            unresolved,
        });
    }
    for e in &mut entries {
        e.cited_by = cited_by.get(e.paper_id.as_str())
            .cloned().unwrap_or_default();
        e.cites.sort();
        e.cites.dedup();
        e.cited_by.sort();
        e.cited_by.dedup();
    }
    let _ = title_by_id;
    entries.sort_by(|a, b| b.cited_by.len().cmp(&a.cited_by.len()).then(a.title.cmp(&b.title)));
    entries
}

/// Best-effort Crossref metadata lookup for a DOI. Never fatal to ingest.
pub async fn enrich_crossref(doi: &str) -> AppResult<PaperReference> {
    let doi = identifiers::normalize_doi(doi)
        .ok_or_else(|| AppError::BadRequest(format!("'{doi}' is not a valid DOI")))?;
    let url = format!("https://api.crossref.org/works/{doi}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("paper-explainer/0.1 (mailto:research@example.com)")
        .build().map_err(|e| AppError::Fetch(e.to_string()))?;
    let resp = client.get(&url).send().await
        .map_err(|e| AppError::Fetch(format!("Crossref lookup failed: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Fetch(format!("Crossref returned {}", resp.status())));
    }
    let body: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Fetch(format!("Crossref response invalid: {e}")))?;
    let msg = &body["message"];

    let title = msg["title"].as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let authors = msg["author"].as_array().map(|a| a.iter()
        .filter_map(|x| {
            let given = x["given"].as_str().unwrap_or("");
            let family = x["family"].as_str().unwrap_or("");
            let name = format!("{given} {family}").trim().to_string();
            if name.is_empty() { None } else { Some(name) }
        }).collect()).unwrap_or_default();
    let year = msg["issued"]["date-parts"][0][0].as_u64()
        .or_else(|| msg["published"]["date-parts"][0][0].as_u64())
        .and_then(|y| u32::try_from(y).ok());
    let arxiv_id = msg["arxiv-id"].as_str()
        .and_then(|s| identifiers::parse_arxiv(s))
        .and_then(|r| r.bare_id);

    Ok(PaperReference {
        id: format!("ref:crossref:{}", make_id("", &doi).trim_start_matches([':', '-'])),
        raw: title.clone().unwrap_or_else(|| doi.clone()),
        title, authors, year, arxiv_id,
        doi: Some(doi), resolved_paper_id: None, enriched: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_arxiv_id_doi_year_and_title() {
        let refs = parse_references(&[
            "1. Vaswani A, Shazeer N, Parmar N. Attention is all you need. NeurIPS 2017. arXiv:1706.03762".into(),
            "2. Smith J. Deep learning. Nature 10.1038/nature14539 (2015).".into(),
        ], "test");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].arxiv_id.as_deref(), Some("1706.03762"));
        assert_eq!(refs[0].year, Some(2017));
        assert!(refs[0].title.as_deref().is_some_and(|t| t.contains("Attention")));
        assert_eq!(refs[1].doi.as_deref(), Some("10.1038/nature14539"));
        assert_eq!(refs[1].year, Some(2015));
    }

    #[test]
    fn resolves_references_when_cited_paper_is_in_corpus() {
        let mut citing = Paper::default();
        citing.id = "a".into();
        citing.references = parse_references(&[
            "Vaswani A. Attention is all you need. arXiv:1706.03762 (2017).".into(),
        ], "a");
        let mut cited = Paper::default();
        cited.id = "b".into();
        cited.arxiv_id = Some("1706.03762".into());
        cited.title = "Attention is all you need".into();

        let mut papers = vec![citing, cited];
        resolve_all(&mut papers);
        assert_eq!(papers[0].references[0].resolved_paper_id.as_deref(), Some("b"));

        let s = summary(&papers);
        let a = s.iter().find(|e| e.paper_id == "a").unwrap();
        let b = s.iter().find(|e| e.paper_id == "b").unwrap();
        assert_eq!(a.cites, vec!["b"]);
        assert_eq!(b.cited_by, vec!["a"]);
    }

    #[test]
    fn references_do_not_resolve_to_the_citing_paper() {
        let mut p = Paper::default();
        p.id = "self".into();
        p.title = "Self citation".into();
        p.references = parse_references(&["Self citation. arXiv:1234.56789 (2020).".into()], "self");
        p.arxiv_id = Some("1234.56789".into());
        let mut papers = vec![p];
        resolve_all(&mut papers);
        assert!(papers[0].references[0].resolved_paper_id.is_none());
    }
}
