//! Orchestrates URL / DOI / PDF / text → parsed document → structured
//! explanation → stored paper.
//!
//! Canonical identifiers are resolved *before* the store is touched so that
//! `2609.10117`, `2609.10117v2`, the arXiv DOI and the `/abs/` and `/pdf/`
//! URLs all map to one paper. A re-import (new version) preserves `added_at`.

use crate::categories::categorize;
use crate::error::{AppError, AppResult};
use crate::explain::explain;
use crate::identifiers::{self, IdentifierKind};
use crate::models::*;
use crate::parse::{fetch_url, parse};
use crate::pdf;
use crate::store::StoreHandle;
use crate::util::{make_id, slug_from_url};
use base64::Engine;
use chrono::Utc;

/// Decode, fetch or otherwise obtain the raw document for an ingest request.
/// Returns the effective URL (used for provenance) and the raw bytes/text.
async fn obtain_document(req: &IngestRequest) -> AppResult<(String, String)> {
    let has_url = req.url.as_ref().is_some_and(|u| !u.trim().is_empty());
    let has_doi = req.doi.as_ref().is_some_and(|d| !d.trim().is_empty());
    let has_pdf = req.pdf.as_ref().is_some_and(|p| !p.trim().is_empty());
    let has_text = req.text.as_ref().is_some_and(|t| !t.trim().is_empty());

    let chosen = [has_url, has_doi, has_pdf, has_text].iter().filter(|x| **x).count();
    if chosen == 0 {
        return Err(AppError::BadRequest(
            "provide one of 'url', 'doi', 'pdf' (base64) or 'text'".into(),
        ));
    }
    if chosen > 1 {
        return Err(AppError::BadRequest(
            "provide only one of 'url', 'doi', 'pdf' or 'text' at a time".into(),
        ));
    }

    if has_pdf {
        let b64 = req.pdf.as_ref().unwrap().trim();
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64))
            .map_err(|e| AppError::BadRequest(format!("invalid base64 PDF data: {e}")))?;
        let name = req.pdf_filename.as_deref().unwrap_or("upload.pdf");
        let url = format!("pdf://{name}");
        let text = pdf::extract_text(&bytes)?;
        return Ok((url, text));
    }

    if has_doi {
        let doi = req.doi.as_ref().unwrap();
        let resolved = identifiers::parse(doi);
        if resolved.kind == IdentifierKind::Arxiv {
            let u = resolved.url.ok_or_else(|| AppError::Internal("arXiv URL missing".into()))?;
            return fetch_url(&u).await;
        }
        let target = identifiers::resolve_doi_url(doi).await?;
        return fetch_url(&target).await;
    }

    if has_url {
        let raw_url = req.url.as_ref().unwrap();
        // Bare arXiv ids / DOIs pasted into the URL box are accepted too.
        let resolved = identifiers::parse(raw_url);
        if resolved.kind == IdentifierKind::Arxiv {
            if let Some(u) = resolved.url { return fetch_url(&u).await; }
        }
        let normalized = crate::parse::normalize_arxiv_url(raw_url);
        return fetch_url(&normalized).await;
    }

    Ok(("pasted://paper".into(), req.text.clone().unwrap()))
}

/// Decide the stable store id and arXiv metadata for an ingest request.
fn identify(req: &IngestRequest, url: &str, title: &str)
    -> (String, Option<String>, Option<String>, Option<String>)
{
    let candidate = req.doi.as_deref()
        .or(req.url.as_deref())
        .unwrap_or(url);
    let resolved = identifiers::parse(candidate);
    let doi = req.doi.as_deref().and_then(identifiers::normalize_doi);

    let (id, arxiv_id, doi, version) = match resolved.kind {
        IdentifierKind::Arxiv => (
            resolved.canonical_id,
            resolved.bare_id,
            doi,
            resolved.version,
        ),
        IdentifierKind::Doi => (
            resolved.canonical_id, None,
            doi.or_else(|| identifiers::normalize_doi(candidate)),
            None,
        ),
        IdentifierKind::Other => {
            let id = if url.contains("arxiv.org") || url.starts_with("2609") {
                slug_from_url(url)
            } else {
                make_id("paper", title)
            };
            (id, None, doi, None)
        }
    };
    (id, arxiv_id, doi, version)
}

pub async fn ingest_paper(req: IngestRequest, store: &StoreHandle) -> AppResult<Paper> {
    let (url, raw) = obtain_document(&req).await?;
    let doc = parse(&url, &raw)?;
    let title = req.title.as_deref().filter(|t| !t.trim().is_empty())
        .unwrap_or(&doc.title).to_string();
    if title.is_empty() {
        return Err(AppError::Parse("could not determine a paper title".into()));
    }

    let (id, arxiv_id, doi, version) = identify(&req, &url, &title);

    if !req.overwrite {
        if let Some(existing) = store.read()?.find_paper(&id) {
            let hint = if let Some(v) = version.as_deref() {
                format!(" (found arXiv version {v}; set overwrite=true to replace it)")
            } else {
                String::new()
            };
            return Err(AppError::BadRequest(format!(
                "paper '{id}' — “{}” — is already in the collection{hint}. Set overwrite=true to re-import it.",
                existing.title
            )));
        }
    }

    let extraction = explain(&doc, &id, &url);
    let year = doc.published.as_ref()
        .and_then(|d| d.chars().take(4).collect::<String>().parse::<u32>().ok());
    let venue = if url.contains("arxiv.org") { Some("arXiv".into()) }
        else if url.starts_with("pdf://") { Some("PDF upload".into()) }
        else { None };
    let source = if url.starts_with("pdf://") { PaperSource::Pdf }
        else if url.starts_with("pasted://") { PaperSource::Manual }
        else { PaperSource::Heuristic };

    // Re-imports keep their original addition date so a newer arXiv version
    // does not jump to the top of the library as if it were a new paper.
    let added_at = if req.overwrite {
        store.read()?.find_paper(&id).map(|p| p.added_at).unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };

    let mut paper = Paper {
        id: id.clone(),
        title,
        authors: doc.authors,
        year,
        url,
        venue,
        abstract_text: doc.abstract_text,
        summary: extraction.summary,
        tags: extraction.tags,
        categories: Vec::new(),
        primary_category: String::new(),
        status: if extraction.experiments.is_empty() { PaperStatus::Failed } else { PaperStatus::Ready },
        source,
        arxiv_id,
        doi,
        version,
        added_at,
        updated_at: Utc::now(),
        hypotheses: extraction.hypotheses,
        experiments: extraction.experiments,
        findings: extraction.findings,
        diagrams: extraction.diagrams,
        workflows: extraction.workflows,
        concepts: extraction.concepts,
        sections: doc.sections,
        references: crate::citations::parse_references(&doc.reference_lines, &id),
        notes: extraction.notes,
        enhancement: None,
        item_provenance: std::collections::HashMap::new(),
    };
    let (categories, primary) = categorize(&paper);
    paper.categories = categories;
    paper.primary_category = primary;

    store.upsert_paper(paper)
}

pub async fn refresh_paper(id: &str, store: &StoreHandle) -> AppResult<Paper> {
    let existing = store.get_paper(id)?;
    if existing.url.is_empty() || existing.url.starts_with("pasted://") || existing.url.starts_with("pdf://") {
        return Err(AppError::BadRequest("pasted and uploaded papers cannot be refreshed from a URL".into()));
    }
    ingest_paper(
        IngestRequest {
            url: Some(existing.url), title: Some(existing.title), overwrite: true,
            ..Default::default()
        },
        store,
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_canonicalises_arxiv_versions() {
        let (id, arxiv_id, _doi, version) = identify(
            &IngestRequest { url: Some("https://arxiv.org/abs/2609.10117v2".into()), ..Default::default() },
            "https://arxiv.org/html/2609.10117",
            "Some title",
        );
        assert_eq!(id, "2609-10117");
        assert_eq!(arxiv_id.as_deref(), Some("2609.10117"));
        assert_eq!(version.as_deref(), Some("v2"));
    }

    #[test]
    fn identify_falls_back_to_title_slug_for_generic_urls() {
        let (id, arxiv_id, _doi, version) = identify(
            &IngestRequest { url: Some("https://example.com/paper".into()), ..Default::default() },
            "https://example.com/paper",
            "A Great Paper",
        );
        assert_eq!(id, make_id("paper", "A Great Paper"));
        assert!(arxiv_id.is_none());
        assert!(version.is_none());
    }
}
