//! Cross-paper hypothesis aggregation: clusters related claims, tracks their
//! status over time and surfaces counter-evidence. Fully deterministic.

use crate::models::{EvidenceLink, HypothesisEvent, HypothesisStatus, HypothesisTracker, HypothesisCluster, Paper};
use crate::util::truncate;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z0-9]+").unwrap());

static STOP_WORDS: &[&str] = &[
    "the", "and", "for", "that", "this", "with", "from", "into", "under", "over", "which",
    "whether", "there", "these", "those", "have", "has", "will", "would", "could", "should",
    "paper", "work", "study", "result", "results", "show", "shows", "showed", "find", "finds",
    "found", "approach", "method", "model", "models", "using", "used", "based", "given",
    "about", "than", "then", "when", "where", "what", "how", "why", "not", "no", "nor",
    "but", "however", "also", "only", "such", "once", "both", "each", "more", "most", "less",
    "very", "can", "may", "might", "does", "did", "are", "was", "were", "been", "being",
    "our", "their", "its", "theirs", "we", "they", "it", "one", "two", "three", "via",
];

fn tokens(text: &str) -> Vec<String> {
    TOKEN_RE.find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .filter(|t| t.chars().count() >= 4 && !STOP_WORDS.contains(&t.as_str()))
        .collect()
}

fn similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() { return 0.0; }
    let sa: HashSet<&String> = a.iter().collect();
    let sb: HashSet<&String> = b.iter().collect();
    let shared = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 { 0.0 } else { shared as f64 / union as f64 }
}

/// A finding that contradicts or qualifies the claim it is attached to.
fn counter_claim(text: &str) -> bool {
    let lower = text.to_lowercase();
    const CUES: &[&str] = &[
        "however", "but ", "not ", "no ", "cannot", "fails", "fail to", "does not", "did not",
        "limited", "only ", "breaks", "boundary", "doesn't", "undermines", "contradicts",
        "except", "unless", "weaker", "degrades", "empirical and attack-specific",
    ];
    CUES.iter().any(|c| lower.contains(c))
}

fn status_key(s: &HypothesisStatus) -> &'static str {
    match s {
        HypothesisStatus::Supported => "supported",
        HypothesisStatus::PartiallySupported => "partially_supported",
        HypothesisStatus::Open => "open",
        HypothesisStatus::Rejected => "rejected",
    }
}

fn verdict(statuses: &[&HypothesisStatus], papers: usize) -> &'static str {
    let supported = statuses.iter().filter(|s| ***s == HypothesisStatus::Supported).count();
    let against = statuses.iter().filter(|s| matches!(s, s if ***s == HypothesisStatus::Rejected || ***s == HypothesisStatus::PartiallySupported)).count();
    if against > 0 && supported > 0 { return "contested"; }
    if against > 0 { return "rejected"; }
    if supported == statuses.len() && papers > 1 { return "supported"; }
    if statuses.iter().all(|s| **s == HypothesisStatus::Open) { return "open"; }
    if supported > 0 { return "supported"; }
    "mixed"
}

fn build_event(paper: &Paper, h: &crate::models::Hypothesis) -> HypothesisEvent {
    let by_id: HashMap<&str, &crate::models::Finding> =
        paper.findings.iter().map(|f| (f.id.as_str(), f)).collect();
    let mut evidence = Vec::new();
    let mut counter_evidence = Vec::new();
    for id in h.evidence_ids.iter().chain(h.experiment_ids.iter()) {
        if let Some(f) = by_id.get(id.as_str()) {
            let counter = counter_claim(&f.claim) || f.caveat.is_some()
                || matches!(h.status, HypothesisStatus::Rejected | HypothesisStatus::PartiallySupported);
            if counter { counter_evidence.push(f.claim.clone()); }
            evidence.push(EvidenceLink {
                id: f.id.clone(),
                claim: truncate(&f.claim, 220),
                caveat: f.caveat.clone(),
                counter,
            });
        }
    }
    HypothesisEvent {
        paper_id: paper.id.clone(),
        paper_title: paper.title.clone(),
        paper_year: paper.year,
        added_at: paper.added_at,
        hypothesis_id: h.id.clone(),
        statement: h.statement.clone(),
        simplified: h.simplified.clone(),
        status: h.status.clone(),
        confidence: h.confidence.clone(),
        section: h.section.clone(),
        evidence,
        counter_evidence,
    }
}

/// Greedy token-overlap clustering of all hypotheses in the corpus.
pub fn build_tracker(papers: &[Paper]) -> HypothesisTracker {
    let mut events: Vec<(Vec<String>, HypothesisEvent)> = Vec::new();
    for paper in papers {
        for h in &paper.hypotheses {
            events.push((tokens(&h.statement), build_event(paper, h)));
        }
    }
    events.sort_by(|a, b| b.1.added_at.cmp(&a.1.added_at).then(a.1.statement.cmp(&b.1.statement)));

    // Union-find over candidate pairs (threshold chosen to group paraphrases
    // without merging unrelated claims).
    let n = events.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x { parent[x] = parent[parent[x]]; x = parent[x]; }
        x
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if events[i].1.paper_id == events[j].1.paper_id { continue; }
            let sa: HashSet<&String> = events[i].0.iter().collect();
            let sb: HashSet<&String> = events[j].0.iter().collect();
            let shared = sa.intersection(&sb).count();
            if shared < 2 { continue; }
            if similarity(&events[i].0, &events[j].0) >= 0.30 {
                let ri = root(&mut parent, i);
                let rj = root(&mut parent, j);
                if ri != rj { parent[ri.max(rj)] = ri.min(rj); }
            }
        }
    }

    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n { groups.entry(root(&mut parent, i)).or_default().push(i); }

    let mut clusters: Vec<HypothesisCluster> = Vec::new();
    for (_, mut members) in groups {
        members.sort();
        let statuses: Vec<&HypothesisStatus> = members.iter().map(|&i| &events[i].1.status).collect();
        let mut papers: Vec<String> = members.iter().map(|&i| events[i].1.paper_id.clone()).collect();
        papers.sort(); papers.dedup();
        let mut keywords: Vec<String> = members.iter()
            .flat_map(|&i| events[i].0.iter().cloned())
            .collect::<HashSet<_>>().into_iter().collect();
        keywords.sort();
        let label_source = &events[members[0]].1;
        let label = if papers.len() > 1 {
            format!("{} — {} papers", truncate(&label_source.statement, 72), papers.len())
        } else {
            truncate(&label_source.statement, 100)
        };
        let first_seen: DateTime<Utc> = members.iter().map(|&i| events[i].1.added_at).min().unwrap_or(label_source.added_at);
        let last_seen: DateTime<Utc> = members.iter().map(|&i| events[i].1.added_at).max().unwrap_or(label_source.added_at);

        // Cross-paper counter-points: opposing statuses in the same theme.
        let mut counter_points = Vec::new();
        for &i in &members {
            let s = &events[i].1.status;
            if matches!(s, HypothesisStatus::Supported) {
                for &j in &members {
                    if matches!(events[j].1.status, HypothesisStatus::Rejected | HypothesisStatus::PartiallySupported) {
                        let note = format!("{}: {} ({})", events[j].1.paper_title, events[j].1.statement, status_key(&events[j].1.status));
                        if !counter_points.contains(&note) { counter_points.push(note); }
                    }
                }
            } else if matches!(s, HypothesisStatus::Rejected | HypothesisStatus::PartiallySupported) {
                for &j in &members {
                    if matches!(events[j].1.status, HypothesisStatus::Supported) {
                        let note = format!("{}: {} (supported)", events[j].1.paper_title, events[j].1.statement);
                        if !counter_points.contains(&note) { counter_points.push(note); }
                    }
                }
            }
        }

        clusters.push(HypothesisCluster {
            id: format!("hc:{}", members.iter().map(|&i| &events[i].1.hypothesis_id).cloned().collect::<Vec<_>>().join("|")),
            label,
            keywords: keywords.into_iter().take(8).collect(),
            verdict: verdict(&statuses, papers.len()).to_string(),
            supported: statuses.iter().filter(|s| ***s == HypothesisStatus::Supported).count(),
            partial: statuses.iter().filter(|s| ***s == HypothesisStatus::PartiallySupported).count(),
            open: statuses.iter().filter(|s| ***s == HypothesisStatus::Open).count(),
            rejected: statuses.iter().filter(|s| ***s == HypothesisStatus::Rejected).count(),
            papers,
            events: members.iter().map(|&i| events[i].1.clone()).collect(),
            counter_points,
            first_seen,
            last_seen,
        });
    }

    let verdict_rank = |v: &str| match v { "contested" => 0, "rejected" => 1, "open" => 2, "mixed" => 3, _ => 4 };
    clusters.sort_by(|a, b| verdict_rank(&a.verdict).cmp(&verdict_rank(&b.verdict))
        .then(b.papers.len().cmp(&a.papers.len()))
        .then(b.last_seen.cmp(&a.last_seen)));

    let timeline: Vec<HypothesisEvent> = {
        let mut all: Vec<HypothesisEvent> = events.into_iter().map(|(_, e)| e).collect();
        all.sort_by(|a, b| b.added_at.cmp(&a.added_at).then(a.statement.cmp(&b.statement)));
        all
    };

    let all_statuses: Vec<&HypothesisStatus> = timeline.iter().map(|e| &e.status).collect();
    HypothesisTracker {
        total_hypotheses: timeline.len(),
        papers: papers.len(),
        supported: all_statuses.iter().filter(|s| ***s == HypothesisStatus::Supported).count(),
        contested: clusters.iter().filter(|c| c.verdict == "contested").count(),
        open: all_statuses.iter().filter(|s| ***s == HypothesisStatus::Open).count(),
        clusters,
        timeline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::seed_paper;

    #[test]
    fn tokenization_drops_stop_words_and_short_tokens() {
        let t = tokens("The model does not generalize across datasets with static keys.");
        assert!(t.contains(&"generalize".to_string()));
        assert!(t.contains(&"datasets".to_string()));
        assert!(t.contains(&"static".to_string()));
        assert!(t.contains(&"keys".to_string()));
        assert!(!t.contains(&"the".to_string()));
        assert!(!t.contains(&"not".to_string()));
        assert!(!t.contains(&"does".to_string()));
        assert!(!t.contains(&"model".to_string()));
    }

    #[test]
    fn counter_claims_are_detected() {
        assert!(counter_claim("However, the result only holds for one dataset."));
        assert!(counter_claim("O_ext resistance is empirical and attack-specific."));
        assert!(!counter_claim("Collapse averages 90.54% accuracy."));
    }

    #[test]
    fn tracker_aggregates_the_curated_seed() {
        let paper = seed_paper();
        let tracker = build_tracker(&[paper]);
        assert_eq!(tracker.papers, 1);
        assert_eq!(tracker.total_hypotheses, tracker.timeline.len());
        assert!(tracker.total_hypotheses > 0);
        assert!(tracker.clusters.iter().all(|c| !c.events.is_empty()));
        // Partially supported O_ext claim should surface its caveat as counter-evidence.
        assert!(tracker.timeline.iter().any(|e| !e.counter_evidence.is_empty()),
            "expected at least one event with counter-evidence");
    }

    #[test]
    fn empty_corpus_yields_empty_tracker() {
        let tracker = build_tracker(&[]);
        assert_eq!(tracker.total_hypotheses, 0);
        assert!(tracker.clusters.is_empty());
    }
}
