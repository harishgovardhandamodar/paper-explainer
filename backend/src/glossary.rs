//! Collection-wide glossary and concept explorer.
//!
//! Concepts are extracted per paper by the deterministic explainer. This
//! module merges them by name across the corpus, writes a plain-language
//! summary for students and derives prerequisite / "where used" links. Nothing
//! here is AI-generated — the same source text drives every view of a paper.

use crate::models::{GlossaryEntry, Paper};
use crate::util::make_id;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static PAREN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\([^)]{4,}\)").unwrap());

const CATEGORY_INTRO: &[(&str, &str)] = &[
    ("primitive", "A basic building block the method is constructed from."),
    ("attack", "A way an adversary tries to break the system's protection."),
    ("metric", "A number used to measure how well something works."),
    ("dataset", "A collection of examples used for training or testing."),
    ("model", "A named machine-learning model."),
    ("concept", "An idea the paper uses to explain its approach."),
];

/// Deterministically rewrite a technical definition into plainer wording.
/// This is intentionally conservative: it removes parenthetical asides and
/// swaps a small set of frequent academic terms, keeping the factual content.
pub fn simplify(definition: &str, category: &str) -> String {
    let without_asides = PAREN_RE.replace_all(definition, "").to_string();
    let mut text = without_asides.trim().to_string();
    for (from, to) in [
        ("utilizes", "uses"), ("utilise", "use"), ("demonstrates", "shows"),
        ("approximately", "about"), ("additionally", "also"), ("therefore", "so"),
        ("prior ", "earlier "), ("obfuscation", "hiding"), ("obfuscated", "hidden"),
        ("surrogate", "stand-in"), ("oracle", "helper"), ("primitive", "building block"),
        ("adversary", "attacker"), ("non-linear", "nonlinear"), ("evaluate", "test"),
        ("evaluation", "testing"), ("methodology", "method"), ("subsumes", "includes"),
    ] {
        text = replace_word(&text, from, to);
    }
    let intro = CATEGORY_INTRO.iter()
        .find(|(c, _)| c.eq_ignore_ascii_case(category))
        .map(|(_, s)| *s)
        .unwrap_or("A term used in this collection.");
    let text = trim_sentence(&text, 170);
    if text.is_empty() { intro.to_string() } else { format!("{intro} {text}") }
}

fn replace_word(text: &str, from: &str, to: &str) -> String {
    let pattern = format!(r"(?i)\b{from}\b");
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, to).to_string(),
        Err(_) => text.to_string(),
    }
}

fn trim_sentence(s: &str, max: usize) -> String {
    let s = s.trim().trim_end_matches('.');
    if s.chars().count() <= max { return s.to_string(); }
    let cut: String = s.chars().take(max.saturating_sub(1)).collect();
    match cut.rfind(|c: char| c == ' ' || c == ',') {
        Some(i) => format!("{}…", &cut[..i]),
        None => format!("{cut}…"),
    }
}

/// Aggregate concepts from every paper into one sorted glossary.
pub fn build(papers: &[Paper]) -> Vec<GlossaryEntry> {
    // name (lowercase) -> entry, merging duplicate concepts across papers.
    let mut by_name: HashMap<String, GlossaryEntry> = HashMap::new();

    for paper in papers {
        for c in &paper.concepts {
            let key = c.name.trim().to_lowercase();
            if key.is_empty() { continue; }
            let entry = by_name.entry(key.clone()).or_insert_with(|| GlossaryEntry {
                id: make_id("glossary", &c.name),
                name: c.name.clone(),
                definition: String::new(),
                simple_definition: String::new(),
                category: c.category.clone(),
                aliases: Vec::new(),
                prerequisites: Vec::new(),
                paper_ids: Vec::new(),
                usage_count: 0,
            });
            // Prefer the longest definition: curated terms are typically
            // richer than the one-line heuristic fallback.
            if c.definition.len() > entry.definition.len() {
                entry.definition = c.definition.clone();
                entry.category = c.category.clone();
                entry.simple_definition = simplify(&c.definition, &c.category);
            }
            if !entry.paper_ids.contains(&paper.id) { entry.paper_ids.push(paper.id.clone()); }
            entry.usage_count += 1;
            for alias in &c.aliases {
                if !entry.aliases.iter().any(|a| a.eq_ignore_ascii_case(alias)) {
                    entry.aliases.push(alias.clone());
                }
            }
        }
    }

    // Acronyms like "TEE" get the expanded phrase from their own definition
    // as an alias, which makes searching by full name work too.
    for entry in by_name.values_mut() {
        if entry.name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            if let Some(first) = entry.definition.split(':').next() {
                let expanded = first.trim();
                if expanded.len() > entry.name.len() + 3
                    && expanded.chars().count() < 90
                    && !entry.aliases.iter().any(|a| a.eq_ignore_ascii_case(expanded)) {
                    entry.aliases.push(expanded.to_string());
                }
            }
        }
    }

    let mut result: Vec<GlossaryEntry> = by_name.into_values().collect();

    // Prerequisites: concepts mentioned in a term's name or definition that
    // come earlier in the teaching order (primitive/concept → model → attack).
    // Compute from an immutable pass first, then apply, to avoid aliasing the
    // vector we are mutating.
    let rank = |cat: &str| match cat.to_lowercase().as_str() {
        "primitive" => 0, "concept" => 1, "dataset" | "model" => 2,
        "metric" => 3, "attack" => 4, _ => 1,
    };
    let mut derived: HashMap<String, Vec<String>> = HashMap::new();
    for entry in &result {
        let key = entry.name.to_lowercase();
        let hay = format!("{} {}", entry.name, entry.definition).to_lowercase();
        let mut prereqs: Vec<(u8, String)> = Vec::new();
        for other in &result {
            let other_key = other.name.to_lowercase();
            if other_key == key || !contains_term(&hay, &other_key) { continue; }
            prereqs.push((rank(&other.category), other.name.clone()));
        }
        prereqs.sort();
        let mut names: Vec<String> = prereqs.into_iter()
            .filter(|(r, _)| *r <= rank(&entry.category))
            .take(4)
            .map(|(_, name)| name)
            .collect();
        names.sort();
        names.dedup();
        derived.insert(key, names);
    }
    for entry in &mut result {
        if let Some(names) = derived.remove(&entry.name.to_lowercase()) {
            entry.prerequisites = names;
        }
    }

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

fn contains_term(hay: &str, term: &str) -> bool {
    if term.chars().count() < 3 { return false; }
    let pattern = format!(r"\b{}\b", regex::escape(term));
    Regex::new(&pattern).map(|r| r.is_match(hay)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Concept, Paper};

    fn paper_with(id: &str, concepts: &[(&str, &str, &str)]) -> Paper {
        let mut p = Paper::default();
        p.id = id.into();
        p.concepts = concepts.iter().map(|(n, d, c)| Concept {
            id: format!("c:{n}"), name: (*n).into(), definition: (*d).into(),
            category: (*c).into(), aliases: Vec::new(),
        }).collect();
        p
    }

    #[test]
    fn merges_concepts_across_papers_and_tracks_usage() {
        let a = paper_with("a", &[("TEE", "Trusted Execution Environment: isolated memory.", "primitive")]);
        let b = paper_with("b", &[("TEE", "Trusted Execution Environment: hardware isolation for code.", "primitive")]);
        let g = build(&[a, b]);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].name, "TEE");
        assert_eq!(g[0].paper_ids, vec!["a", "b"]);
        assert_eq!(g[0].usage_count, 2);
        assert!(g[0].aliases.iter().any(|x| x.contains("Trusted Execution")));
        assert!(g[0].simple_definition.contains("building block"));
    }

    #[test]
    fn derives_prerequisites_from_mentions_in_definitions() {
        let a = paper_with("a", &[
            ("TEE", "Trusted Execution Environment: isolated execution.", "primitive"),
            ("Collapse", "An extraction attack that needs access to a TEE side channel.", "attack"),
        ]);
        let g = build(&[a]);
        let collapse = g.iter().find(|e| e.name == "Collapse").unwrap();
        assert!(collapse.prerequisites.iter().any(|p| p == "TEE"));
    }

    #[test]
    fn simplify_strips_parentheticals_and_jargon() {
        let s = simplify("Obfuscation (hiding weights) utilizes a helper and demonstrates results.", "primitive");
        assert!(!s.contains("hiding weights"));
        assert!(s.contains("uses"));
        assert!(s.contains("shows"));
        assert!(s.starts_with("A basic building block"));
    }
}
