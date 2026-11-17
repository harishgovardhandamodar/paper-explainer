//! Orchestrates URL → parsed document → structured explanation → stored paper.

use crate::error::{AppError, AppResult};
use crate::explain::explain;
use crate::models::*;
use crate::parse::{fetch_url, parse};
use crate::store::StoreHandle;
use crate::util::{make_id, slug_from_url};
use chrono::Utc;

pub async fn ingest_paper(req: IngestRequest, store: &StoreHandle) -> AppResult<Paper> {
    let has_url = req.url.as_ref().is_some_and(|u| !u.trim().is_empty());
    let has_text = req.text.as_ref().is_some_and(|t| !t.trim().is_empty());
    if !has_url && !has_text {
        return Err(AppError::BadRequest(
            "provide either a 'url' (e.g. https://arxiv.org/html/2609.10117v2) or 'text'".into(),
        ));
    }

    let (url, raw) = if has_url {
        let u = req.url.clone().unwrap();
        fetch_url(&u).await?
    } else {
        ("pasted://paper".into(), req.text.clone().unwrap())
    };

    let doc = parse(&url, &raw)?;
    let title = req.title.filter(|t| !t.trim().is_empty()).unwrap_or(doc.title.clone());
    if title.is_empty() {
        return Err(AppError::Parse("could not determine a paper title".into()));
    }

    let id = if url.contains("arxiv.org") || url.starts_with("2609") {
        slug_from_url(&url)
    } else {
        make_id("paper", &title)
    };

    if !req.overwrite && store.read()?.find_paper(&id).is_some() {
        return Err(AppError::BadRequest(format!(
            "paper '{id}' is already in the collection. Set overwrite=true to re-import it."
        )));
    }

    let extraction = explain(&doc, &id, &url);
    let year = doc.published.as_ref().and_then(|d| d.chars().take(4).collect::<String>().parse::<u32>().ok());
    let venue = if url.contains("arxiv.org") { Some("arXiv".into()) } else { None };
    let now = Utc::now();
    let source = if has_text { PaperSource::Manual } else { PaperSource::Heuristic };

    let paper = Paper {
        id: id.clone(), title, authors: doc.authors, year, url, venue,
        abstract_text: doc.abstract_text,
        summary: extraction.summary,
        tags: extraction.tags,
        status: if extraction.experiments.is_empty() { PaperStatus::Failed } else { PaperStatus::Ready },
        source,
        added_at: now,
        updated_at: now,
        hypotheses: extraction.hypotheses,
        experiments: extraction.experiments,
        findings: extraction.findings,
        diagrams: extraction.diagrams,
        workflows: extraction.workflows,
        concepts: extraction.concepts,
        sections: doc.sections,
        notes: extraction.notes,
        enhancement: None,
    };

    store.upsert_paper(paper)
}

pub async fn refresh_paper(id: &str, store: &StoreHandle) -> AppResult<Paper> {
    let existing = store.get_paper(id)?;
    if existing.url.is_empty() || existing.url.starts_with("pasted://") {
        return Err(AppError::BadRequest("pasted papers cannot be refreshed from a URL".into()));
    }
    ingest_paper(
        IngestRequest { url: Some(existing.url), title: Some(existing.title), overwrite: true, text: None },
        store,
    ).await
}
