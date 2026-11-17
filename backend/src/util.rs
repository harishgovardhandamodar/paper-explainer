//! Small text helpers shared by the parser and the explainer.

use regex::Regex;
use std::sync::LazyLock;

static CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Parenthesized author/year groups such as "(Zhang et al., 2024a)".
    Regex::new(r"\([^()]{0,200}\b(?:19|20)\d{2}[a-z]?\b[^()]{0,200}\)").unwrap()
});
static CITE_NUM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\s*\d+(?:[,\s\u{2013}\u{2014}-]+\d+)*\s*\]").unwrap());
static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static LATEX_TEX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"annotation\s+encoding="application/x-tex"[^>]*>[^<]*"#).unwrap());

/// Collapse whitespace and strip HTML-ish noise. Does not remove math content.
pub fn clean(input: &str) -> String {
    let no_tags = TAG_RE.replace_all(input, " ");
    let no_tex = LATEX_TEX_RE.replace_all(&no_tags, " ");
    let s = html_escape::decode_html_entities(&no_tex);
    WS_RE.replace_all(&s, " ").trim().to_string()
}

/// Remove inline citations and reference markers, leaving readable prose.
pub fn prose(input: &str) -> String {
    let s = clean(input);
    let s = CITATION_RE.replace_all(&s, "");
    let s = CITE_NUM_RE.replace_all(&s, "");
    // Leftover dangling separators from removed citations.
    let s = Regex::new(r"\s{2,}").unwrap().replace_all(&s, " ");
    let s = s.replace(" ,", ",").replace(" .", ".").replace(" ;", ";");
    s.trim().to_string()
}

/// Turn a formal sentence into a friendlier one with light, deterministic edits.
pub fn simplify(input: &str) -> String {
    let mut s = prose(input);
    let swaps = [
        ("utilize", "use"), ("utilizes", "uses"), ("demonstrate that", "show that"),
        ("demonstrates that", "shows that"), ("therefore", "so"), ("however", "but"),
        ("moreover", "also"), ("furthermore", "also"), ("nevertheless", "still"),
        ("subsequently", "then"), ("prior to", "before"), ("in order to", "to"),
        ("aforementioned", "this"), ("approximately", "about"), ("regarding", "about"),
        ("facilitate", "help"), ("facilitates", "helps"), ("numerous", "many"),
        ("insufficient", "not enough"), ("endeavor", "attempt"), ("methodology", "method"),
        ("instantiation", "version"), ("adjacent", "neighboring"),
    ];
    for (from, to) in swaps {
        s = replace_word_boundary(&s, from, to);
    }
    // Join fragments created by citation removal.
    s = s.replace("  ", " ");
    capitalize_first(&s)
}

fn replace_word_boundary(input: &str, from: &str, to: &str) -> String {
    let re = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(from))).unwrap();
    re.replace_all(input, to).to_string()
}

pub fn capitalize_first(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn truncate(input: &str, max: usize) -> String {
    if input.chars().count() <= max { return input.to_string(); }
    let mut out: String = input.chars().take(max).collect();
    while !out.is_empty() && !out.ends_with([' ', '.', ',', ';', ')']) { out.pop(); }
    if out.is_empty() { out = input.chars().take(max).collect(); }
    format!("{}…", out.trim_end())
}

pub fn sentences(input: &str) -> Vec<String> {
    input
        .split(|c| c == '.' || c == '!' || c == '?')
        .map(clean)
        .filter(|s| s.chars().count() > 24)
        .map(|s| capitalize_first(&s))
        .collect()
}

/// Stable, human-readable-ish identifier.
pub fn make_id(prefix: &str, raw: &str) -> String {
    let slug: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = Regex::new("-+").unwrap().replace_all(&slug, "-");
    let slug = slug.trim_matches('-').chars().take(64).collect::<String>();
    format!("{}:{}", prefix, if slug.is_empty() { "item" } else { &slug })
}

/// A short unique suffix so two papers can contribute the same concept name.
pub fn make_scoped_id(prefix: &str, paper_id: &str, raw: &str) -> String {
    let base = make_id(prefix, raw);
    let short: String = paper_id.chars().filter(|c| c.is_ascii_alphanumeric()).take(10).collect();
    format!("{}@{}", base, if short.is_empty() { "x" } else { &short })
}

pub fn slug_from_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/').trim_end_matches(".html");
    let last = trimmed.rsplit('/').next().unwrap_or("paper");
    let last = last.split('?').next().unwrap_or(last);
    let id: String = last.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '-' }).collect();
    if id.is_empty() { "paper".into() } else { id.replace(".", "-") }
}

pub fn first_nonempty(values: &[String]) -> Option<String> {
    values.iter().map(|v| clean(v)).find(|v| !v.is_empty())
}

/// Extract the first N percentages / ratios, useful as auto-detected evidence.
pub fn extract_numbers(input: &str) -> Vec<String> {
    let re = Regex::new(r"\b\d+(?:\.\d+)?\s*(?:%|×|x\b|percent|percentage points|pp\b)").unwrap();
    let mut out = Vec::new();
    for m in re.find_iter(input) {
        let v = clean(m.as_str());
        if !out.contains(&v) { out.push(v); }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_citations() {
        let s = "We show accuracy (Zhang et al., 2024a) improves [3, 5].";
        assert_eq!(prose(s), "We show accuracy improves.");
    }

    #[test]
    fn slugs_urls() {
        assert_eq!(slug_from_url("https://arxiv.org/html/2609.10117v2"), "2609-10117v2");
    }

    #[test]
    fn extracts_numbers() {
        let n = extract_numbers("It reaches 1.45× the baseline and drops 27.4 percentage points, i.e. 62.23%.");
        assert!(n.iter().any(|x| x.contains("1.45")));
        assert!(n.iter().any(|x| x.contains("62.23")));
    }
}
