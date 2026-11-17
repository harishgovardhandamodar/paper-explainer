//! Fetch and parse paper documents (arXiv HTML, generic HTML, or pasted text).

use crate::error::{AppError, AppResult};
use crate::models::PaperSection;
use crate::util::{clean, first_nonempty, prose};
use ego_tree::NodeRef;
use scraper::{Html, Selector};
use scraper::node::{Element, Node};
use scraper::ElementRef;

#[derive(Debug, Clone, Default)]
pub struct ParsedDocument {
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub keywords: Vec<String>,
    pub published: Option<String>,
    pub sections: Vec<PaperSection>,
    pub is_html: bool,
}

/// Normalize user input into a fetchable URL.
pub fn normalize_arxiv_url(input: &str) -> String {
    let url = input.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        if url.contains("arxiv.org/abs/") {
            return url.replacen("/abs/", "/html/", 1);
        }
        if url.contains("arxiv.org/pdf/") {
            let u = url.replacen("/pdf/", "/html/", 1);
            return u.trim_end_matches(".pdf").to_string();
        }
        return url.to_string();
    }
    // Bare arXiv id, e.g. 2609.10117 or 2609.10117v2
    if input.chars().filter(|c| *c == '.').count() == 1
        && input.chars().take(4).all(|c| c.is_ascii_digit())
    {
        return format!("https://arxiv.org/html/{}", url);
    }
    format!("https://{}", url)
}

pub async fn fetch_url(input: &str) -> AppResult<(String, String)> {
    let url = normalize_arxiv_url(input);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .user_agent("paper-explainer/0.1 (+https://github.com/paper-explainer)")
        .build()
        .map_err(|e| AppError::Fetch(e.to_string()))?;
    let resp = client.get(&url).send().await.map_err(|e| AppError::Fetch(format!("{}: {}", url, e)))?;
    if !resp.status().is_success() {
        return Err(AppError::Fetch(format!("{} returned HTTP {}", url, resp.status())));
    }
    let body = resp.text().await.map_err(|e| AppError::Fetch(e.to_string()))?;
    if body.trim().is_empty() {
        return Err(AppError::Fetch(format!("{} returned an empty document", url)));
    }
    Ok((url, body))
}

pub fn parse(input: &str, raw: &str) -> AppResult<ParsedDocument> {
    let looks_like_html = raw.trim_start().starts_with('<') || raw.contains("<html");
    if !looks_like_html {
        return Ok(parse_plain_text(input, raw));
    }
    parse_html(input, raw)
}

fn parse_html(input: &str, raw: &str) -> AppResult<ParsedDocument> {
    let doc = Html::parse_document(raw);
    let meta_sel = Selector::parse("meta").map_err(|e| AppError::Parse(e.to_string()))?;
    let mut meta: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for m in doc.select(&meta_sel) {
        if let (Some(k), Some(v)) = (m.value().attr("name").or_else(|| m.value().attr("property")), m.value().attr("content")) {
            meta.insert(k.to_lowercase(), v.to_string());
        }
    }

    let title = first_nonempty(&[
        meta.get("citation_title").cloned().unwrap_or_default(),
        select_text(&doc, "h1.ltx_title_document"),
        select_text(&doc, "article h1"),
        select_text(&doc, "h1"),
        select_text(&doc, "title"),
        input.to_string(),
    ]).unwrap_or_else(|| "Untitled paper".into());

    let authors: Vec<String> = {
        let mut a: Vec<String> = doc
            .select(&Selector::parse("meta[name=\"citation_author\"]").unwrap())
            .filter_map(|m| m.value().attr("content").map(clean))
            .filter(|s| !s.is_empty())
            .collect();
        if a.is_empty() {
            a = doc
                .select(&Selector::parse(".ltx_personname, .author, [rel=author]").unwrap())
                .take(20)
                .map(|e| clean(&e.text().collect::<String>()))
                .filter(|s| !s.is_empty() && s.len() < 80)
                .collect();
        }
        a
    };

    let abstract_text = first_nonempty(&[
        meta.get("citation_abstract").cloned().unwrap_or_default(),
        meta.get("description").cloned().unwrap_or_default(),
        abstract_from_sections(&doc).unwrap_or_default(),
    ]).map(|s| prose(&s)).unwrap_or_default();

    let keywords: Vec<String> = meta
        .get("keywords")
        .or_else(|| meta.get("citation_keywords"))
        .map(|s| s.split(|c| c == ',' || c == ';').map(clean).filter(|k| !k.is_empty()).collect())
        .unwrap_or_default();

    let published = meta.get("citation_date")
        .or_else(|| meta.get("citation_publication_date"))
        .or_else(|| meta.get("article:published_time"))
        .cloned();

    let mut sections = structured_sections(&doc);
    if sections.is_empty() {
        sections = heading_sections(&doc);
    }

    Ok(ParsedDocument {
        title: clean(&title), authors, abstract_text, keywords, published, sections, is_html: true,
    })
}

fn parse_plain_text(input: &str, raw: &str) -> ParsedDocument {
    let lines: Vec<&str> = raw.lines().collect();
    let title = lines.first().map(|l| clean(l)).unwrap_or_default();
    let title = if title.is_empty() { clean(input) } else { title };
    let title = if title.is_empty() { "Untitled paper".into() } else { title };

    let mut sections = Vec::new();
    let mut current: Option<PaperSection> = None;
    let mut buffer: Vec<String> = Vec::new();
    let heading_re = regex::Regex::new(r"^\s*(\d+(?:\.\d+)*\.?\s+)?(Introduction|Background|Threat Model|Method|Methodology|Approach|System|Design|Attack|Evaluation|Experiments?|Results?|Discussion|Related Work|Conclusion|Abstract)\b").unwrap();

    for line in lines {
        let t = clean(line);
        if t.is_empty() { continue; }
        if heading_re.is_match(&t) {
            if let Some(mut sec) = current.take() {
                sec.paragraphs = std::mem::take(&mut buffer);
                sections.push(sec);
            }
            current = Some(PaperSection { heading: t.clone(), level: 2, paragraphs: Vec::new() });
        } else if current.is_some() {
            buffer.push(t);
        }
    }
    if let Some(mut sec) = current {
        sec.paragraphs = buffer;
        sections.push(sec);
    }
    if sections.is_empty() {
        sections.push(PaperSection {
            heading: "Full text".into(), level: 1,
            paragraphs: raw.split("\n\n").map(clean).filter(|s| s.len() > 40).collect(),
        });
    }

    ParsedDocument {
        title, authors: Vec::new(), abstract_text: String::new(), keywords: Vec::new(),
        published: None, sections, is_html: false,
    }
}

fn abstract_from_sections(doc: &Html) -> Option<String> {
    let sec_sel = Selector::parse("section").ok()?;
    for sec in doc.select(&sec_sel) {
        let heading = sec.child_elements().find(|e| matches!(e.value().name(), "h1" | "h2" | "h3" | "h4"));
        if let Some(h) = heading {
            let t = clean(&h.text().collect::<String>());
            if t.to_lowercase().contains("abstract") {
                let text = clean(&sec.text().collect::<String>());
                let text = text.replacen(&t, "", 1);
                if text.len() > 40 { return Some(text); }
            }
        }
    }
    None
}

/// Structured extraction for LaTeXML / arXiv-style documents.
fn structured_sections(doc: &Html) -> Vec<PaperSection> {
    let sec_sel = match Selector::parse("section") { Ok(s) => s, Err(_) => return Vec::new() };
    let para_sel = match Selector::parse("div.ltx_para, p.ltx_p, p") { Ok(s) => s, Err(_) => return Vec::new() };
    let mut out = Vec::new();

    for sec in doc.select(&sec_sel) {
        let heading = sec.child_elements().find(|e| matches!(e.value().name(), "h1" | "h2" | "h3" | "h4" | "h5"));
        let Some(heading) = heading else { continue; };
        let title = clean(&heading.text().collect::<String>());
        if title.is_empty() || title.eq_ignore_ascii_case("references") { continue; }
        let level = heading.value().name().trim_start_matches('h').parse::<u8>().unwrap_or(2).max(1);

        let mut paragraphs = Vec::new();
        for p in sec.select(&para_sel) {
            // Only paragraphs owned by this section, not by nested subsections.
            if !nearest_section(*p).is_some_and(|s| ptr_eq(&s, &sec)) { continue; }
            let text = prose(&p.text().collect::<String>());
            if text.chars().count() > 30 { paragraphs.push(text); }
        }
        if !paragraphs.is_empty() || title.chars().count() > 0 {
            out.push(PaperSection { heading: title, level, paragraphs });
        }
    }
    out
}

/// Fallback for arbitrary HTML: walk headings and grab following sibling text.
fn heading_sections(doc: &Html) -> Vec<PaperSection> {
    let sel = match Selector::parse("h1, h2, h3, h4, h5") { Ok(s) => s, Err(_) => return Vec::new() };
    let mut out = Vec::new();
    for h in doc.select(&sel) {
        let title = clean(&h.text().collect::<String>());
        if title.is_empty() { continue; }
        let level = h.value().name().trim_start_matches('h').parse::<u8>().unwrap_or(2).max(1);
        let mut paragraphs = Vec::new();
        let mut node = h.next_sibling();
        while let Some(n) = node {
            if let Some(el) = ElementRef::wrap(n) {
                if matches!(el.value().name(), "h1" | "h2" | "h3" | "h4" | "h5") { break; }
                if matches!(el.value().name(), "p" | "li" | "blockquote") {
                    let text = prose(&el.text().collect::<String>());
                    if text.chars().count() > 30 { paragraphs.push(text); }
                }
            }
            node = n.next_sibling();
        }
        if !paragraphs.is_empty() {
            out.push(PaperSection { heading: title, level, paragraphs });
        }
    }
    out
}

fn nearest_section<'a>(node: NodeRef<'a, Node>) -> Option<ElementRef<'a>> {
    let mut cur = node;
    loop {
        let parent = cur.parent()?;
        if let Some(el) = ElementRef::wrap(parent) {
            if el.value().name() == "section" { return Some(el); }
        }
        cur = parent;
    }
}

fn ptr_eq(a: &ElementRef<'_>, b: &ElementRef<'_>) -> bool {
    std::ptr::eq(a.value() as *const Element, b.value() as *const Element)
}

fn select_text(doc: &Html, selector: &str) -> String {
    let sel = match Selector::parse(selector) { Ok(s) => s, Err(_) => return String::new() };
    doc.select(&sel).next().map(|e| clean(&e.text().collect::<String>())).unwrap_or_default()
}

#[allow(dead_code)]
fn node_is_element(node: &Node) -> bool {
    matches!(node, Node::Element(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_arxiv_urls() {
        assert_eq!(normalize_arxiv_url("2609.10117v2"), "https://arxiv.org/html/2609.10117v2");
        assert_eq!(normalize_arxiv_url("https://arxiv.org/abs/2609.10117"), "https://arxiv.org/html/2609.10117");
        assert_eq!(normalize_arxiv_url("https://arxiv.org/pdf/2609.10117v2.pdf"), "https://arxiv.org/html/2609.10117v2");
    }

    #[test]
    fn parses_minimal_html() {
        let html = r#"<html><head><meta name="citation_title" content="A Test Paper"><meta name="citation_author" content="Ada Lovelace"></head><body><section><h2>1. Introduction</h2><div class="ltx_para"><p class="ltx_p">This is a long enough sentence to be kept as a paragraph in the parsed document for testing.</p></div></section></body></html>"#;
        let p = parse("url", html).unwrap();
        assert_eq!(p.title, "A Test Paper");
        assert_eq!(p.authors, vec!["Ada Lovelace"]);
        assert!(p.sections.iter().any(|s| s.heading.contains("Introduction")));
    }
}
