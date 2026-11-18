//! Canonical identifier handling for arXiv papers and DOIs.
//!
//! The dashboard must treat `2609.10117`, `2609.10117v2`,
//! `https://arxiv.org/abs/2609.10117v2` and the arXiv DOI
//! `10.48550/arXiv.2609.10117` as *one* paper. Everything here is
//! deterministic string handling plus an optional redirect lookup for
//! non-arXiv DOIs.

use crate::error::{AppError, AppResult};
use regex::Regex;
use std::sync::LazyLock;

static VERSION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)v(\d+)$").unwrap());
static NEW_ID_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{4})\.(\d{4,5})$").unwrap());
static OLD_ID_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([a-z][a-z-]*)/(\d{7})$").unwrap());
static DOI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^10\.\d{4,9}/\S+$").unwrap());
static ARXIV_DOI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^10\.48550/arxiv[./:]?(.+)$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierKind {
    Arxiv,
    Doi,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIdentifier {
    pub kind: IdentifierKind,
    /// Stable id used as the primary key in the store.
    pub canonical_id: String,
    /// Raw arXiv id without its version, e.g. `2609.10117`.
    pub bare_id: Option<String>,
    /// Version suffix such as `v2`, when present.
    pub version: Option<String>,
    /// Fetchable document URL, when one can be constructed without a lookup.
    pub url: Option<String>,
}

impl ResolvedIdentifier {
    pub fn other() -> Self {
        Self {
            kind: IdentifierKind::Other, canonical_id: String::new(),
            bare_id: None, version: None, url: None,
        }
    }
}

fn split_version(s: &str) -> (String, Option<String>) {
    if let Some(caps) = VERSION_RE.captures(s) {
        let whole = caps.get(0).unwrap().as_str();
        let bare = s[..s.len() - whole.len()].to_string();
        (bare, Some(whole.to_ascii_lowercase()))
    } else {
        (s.to_string(), None)
    }
}

/// Recognise an arXiv id in any of its common written forms and return its
/// canonical, version-free store id plus the (optional) version.
pub fn parse_arxiv(input: &str) -> Option<ResolvedIdentifier> {
    let raw = input.trim().trim_end_matches('/').trim();
    if raw.is_empty() { return None; }

    // Strip common prefixes and URL scaffolding.
    let mut candidate = raw
        .replace("arxiv:", "")
        .replace("arXiv:", "")
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string();

    if let Some(pos) = candidate.find("arxiv.org/") {
        candidate = candidate[pos + "arxiv.org/".len()..].to_string();
    } else if candidate.contains('/') && !candidate.starts_with("www.") {
        // A bare old-style id looks like `cs/0701001v3`: an alphabetic archive
        // prefix followed by digits. Anything else is a non-arxiv URL path,
        // which the caller can hand to DOI resolution.
        let parts: Vec<&str> = candidate.split('/').collect();
        let old_style_bare = parts.len() == 2
            && parts[0].chars().all(|c| c.is_ascii_lowercase() || c == '-')
            && parts[1].chars().next().is_some_and(|c| c.is_ascii_digit());
        if !old_style_bare { return None; }
    }

    let segments: Vec<&str> = candidate.split('/').filter(|s| !s.is_empty()).collect();
    let id_part = match segments.as_slice() {
        [kind, id, ..] if ["abs", "html", "pdf", "format"].contains(kind) => {
            let mut id = (*id).to_string();
            if id.to_ascii_lowercase().ends_with(".pdf") {
                id.truncate(id.len() - 4);
            }
            id
        }
        // Old-style bare id such as `cs/0701001v3`.
        [archive, id]
            if archive.chars().all(|c| c.is_ascii_lowercase() || c == '-')
                && id.chars().next().is_some_and(|c| c.is_ascii_digit()) =>
        {
            format!("{}/{}", archive, id)
        }
        [id] => (*id).to_string(),
        _ => return None,
    };

    // A bare id may be followed by query parameters after the path split.
    let id_part = id_part.split('?').next().unwrap_or(&id_part).to_string();
    let (bare, version) = split_version(&id_part);

    let canonical = if NEW_ID_RE.is_match(&bare) {
        bare.replace('.', "-")
    } else if OLD_ID_RE.is_match(&bare.to_ascii_lowercase()) {
        bare.to_ascii_lowercase().replace('/', "-")
    } else {
        return None;
    };

    let html_url = if OLD_ID_RE.is_match(&bare.to_ascii_lowercase()) {
        format!("https://arxiv.org/abs/{}", bare)
    } else {
        format!("https://arxiv.org/html/{}", bare)
    };

    Some(ResolvedIdentifier {
        kind: IdentifierKind::Arxiv,
        canonical_id: canonical,
        bare_id: Some(bare),
        version,
        url: Some(html_url),
    })
}

/// Normalise a DOI string, handling `doi:` prefixes and resolver URLs.
pub fn normalize_doi(input: &str) -> Option<String> {
    let s = input.trim();
    let s = s.strip_prefix("doi:").or_else(|| s.strip_prefix("DOI:")).unwrap_or(s);
    let s = s
        .replace("https://doi.org/", "")
        .replace("http://doi.org/", "")
        .replace("https://dx.doi.org/", "")
        .replace("http://dx.doi.org/", "")
        .trim_end_matches(['.', ',', ')'])
        .trim()
        .to_string();
    if DOI_RE.is_match(&s) { Some(s) } else { None }
}

/// Resolve any user-supplied identifier without performing network I/O.
/// Returns `Other` for things that are neither arXiv ids nor DOIs.
pub fn parse(input: &str) -> ResolvedIdentifier {
    if let Some(arxiv) = parse_arxiv(input) { return arxiv; }

    if let Some(doi) = normalize_doi(input) {
        if let Some(caps) = ARXIV_DOI_RE.captures(&doi) {
            let inner = caps.get(1).unwrap().as_str().replace("%2F", "/");
            if let Some(arxiv) = parse_arxiv(&inner) { return arxiv; }
        }
        let canonical = doi.to_lowercase().chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        return ResolvedIdentifier {
            kind: IdentifierKind::Doi,
            canonical_id: format!("doi:{canonical}"),
            bare_id: None, version: None,
            url: Some(format!("https://doi.org/{doi}")),
        };
    }

    ResolvedIdentifier::other()
}

/// Follow a DOI's resolver redirect to the publisher/landing page. arXiv DOIs
/// are handled locally by `parse`, so this is only for external DOIs.
pub async fn resolve_doi_url(doi: &str) -> AppResult<String> {
    if let Some(arxiv) = parse_arxiv(doi) {
        if let Some(url) = arxiv.url { return Ok(url); }
    }
    let doi = normalize_doi(doi)
        .ok_or_else(|| AppError::BadRequest(format!("'{doi}' is not a valid DOI")))?;
    let target = format!("https://doi.org/{doi}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("paper-explainer/0.1 (+https://github.com/paper-explainer)")
        .build()
        .map_err(|e| AppError::Fetch(e.to_string()))?;
    let resp = client.get(&target).send().await
        .map_err(|e| AppError::Fetch(format!("{target}: {e}")))?;
    let final_url = resp.url().to_string();
    Ok(final_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arxiv_versions_collapse_to_one_canonical_id() {
        let a = parse("2609.10117");
        let b = parse("2609.10117v2");
        let c = parse("https://arxiv.org/abs/2609.10117v2");
        let d = parse("https://arxiv.org/html/2609.10117v2");
        let e = parse("https://arxiv.org/pdf/2609.10117v2.pdf");
        assert_eq!(a.canonical_id, "2609-10117");
        assert_eq!(a.canonical_id, b.canonical_id);
        assert_eq!(a.canonical_id, c.canonical_id);
        assert_eq!(a.canonical_id, d.canonical_id);
        assert_eq!(a.canonical_id, e.canonical_id);
        assert_eq!(b.version.as_deref(), Some("v2"));
        assert!(a.version.is_none());
        assert_eq!(b.url.as_deref(), Some("https://arxiv.org/html/2609.10117"));
    }

    #[test]
    fn old_style_arxiv_ids_are_canonicalised() {
        let a = parse("cs/0701001v3");
        assert_eq!(a.canonical_id, "cs-0701001");
        assert_eq!(a.version.as_deref(), Some("v3"));
        assert_eq!(a.url.as_deref(), Some("https://arxiv.org/abs/cs/0701001"));
    }

    #[test]
    fn arxiv_dois_resolve_to_the_same_paper() {
        let doi = parse("doi:10.48550/arXiv.2609.10117v2");
        assert_eq!(doi.canonical_id, "2609-10117");
        assert_eq!(doi.version.as_deref(), Some("v2"));
        assert_eq!(doi.url.as_deref(), Some("https://arxiv.org/html/2609.10117"));
        let plain = parse("https://doi.org/10.1000/abc.def");
        assert_eq!(plain.kind, IdentifierKind::Doi);
        assert_eq!(plain.canonical_id, "doi:10-1000-abc-def");
    }

    #[test]
    fn non_identifiers_return_other() {
        assert_eq!(parse("some random title").kind, IdentifierKind::Other);
        assert_eq!(parse("https://example.com/paper").kind, IdentifierKind::Other);
        assert!(parse_arxiv("https://example.com/abs/2609.10117").is_none());
    }
}
