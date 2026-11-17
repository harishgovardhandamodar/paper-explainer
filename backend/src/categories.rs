//! Deterministic subject categorization for the paper library.
//!
//! A small, stable taxonomy assigned from a paper's own text. Categories power
//! faceted browsing and stay useful for a very large corpus because they need
//! no LLM and are computed once (with a backfill for papers saved before this
//! feature existed). Assignment is intentionally explainable: each category is
//! a weighted bag of keywords, a paper can hold up to three, and the highest
//! scoring one becomes its primary category.

use crate::models::{CategoryDef, Paper};

/// One taxonomy entry. `id` is stored on papers and used in API filters, so it
/// must stay stable. `tone` maps to a badge colour in the frontend.
struct CategorySpec {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    tone: &'static str,
    keywords: &'static [&'static str],
}

static CATEGORIES: &[CategorySpec] = &[
    CategorySpec {
        id: "security-privacy", label: "Security & Privacy", tone: "rose",
        description: "Attacks, defences, cryptography, obfuscation and privacy",
        keywords: &["security", "privacy", "attack", "adversarial", "encrypt", "cryptograph",
            "authentication", "vulnerab", "exploit", "obfuscation", "tee", "trusted execution",
            "model extraction", "membership inference", "malware", "intrusion", "threat",
            "defense", "defence", "side-channel", "backdoor", "jailbreak", "enclave"],
    },
    CategorySpec {
        id: "machine-learning", label: "Machine Learning", tone: "violet",
        description: "Model training, architectures, optimisation and evaluation",
        keywords: &["machine learning", "deep learning", "neural network", "training",
            "gradient", "reinforcement learning", "transformer", "fine-tun", "generalization",
            "optimizer", "learning rate", "neural network", "convolutional", "embedding",
            "epoch", "loss function"],
    },
    CategorySpec {
        id: "nlp", label: "Natural Language", tone: "sky",
        description: "Language models, text, translation and retrieval",
        keywords: &["natural language", "language model", "llm", " nlp", "text", "question answering",
            "translation", "tokeniz", "prompt", "retrieval-augmented", "summariz", "chatbot",
            "corpus", "word embedding", "on-device llm"],
    },
    CategorySpec {
        id: "vision", label: "Computer Vision", tone: "amber",
        description: "Images, video, detection, segmentation and recognition",
        keywords: &["computer vision", "image", "video", "visual", "object detection",
            "segmentation", "cnn", "vision transformer", "image classification", "3d scene",
            "optical", "pixel"],
    },
    CategorySpec {
        id: "systems", label: "Systems & Hardware", tone: "green",
        description: "Operating systems, kernels, performance and accelerators",
        keywords: &["operating system", "kernel", "memory", "latency", "throughput", "hardware",
            "gpu", "cpu", "enclave", "scheduling", "microbenchmark", "runtime", "overhead",
            "systems", "architecture", "offloading"],
    },
    CategorySpec {
        id: "distributed", label: "Distributed & Edge", tone: "blue",
        description: "Consensus, replication, cloud/edge and peer-to-peer systems",
        keywords: &["distributed", "consensus", "replication", "blockchain", "peer-to-peer",
            "cluster", "fog", "edge computing", "cloud", "p2p", "federated", "network protocol"],
    },
    CategorySpec {
        id: "data-knowledge", label: "Data & Knowledge", tone: "teal",
        description: "Datasets, databases, indexing and knowledge graphs",
        keywords: &["dataset", "database", "data mining", "knowledge graph", "indexing",
            "query", "sql", "data management", "graph database", "data pipeline", "retrieval",
            "data extraction"],
    },
    CategorySpec {
        id: "theory-methods", label: "Theory & Methods", tone: "indigo",
        description: "Algorithms, proofs, complexity and formal methods",
        keywords: &["algorithm", "proof", "theorem", "complexity", "optimization", "theoretical",
            "bound", "formal verification", "mathematical", "systematization", "framework",
            "taxonomy"],
    },
    CategorySpec {
        id: "hci", label: "Human & Society", tone: "pink",
        description: "User studies, interaction, usability and social impact",
        keywords: &["user study", "human", "usability", "interaction", "interface",
            "user experience", "accessibility", "survey", "participants", "user-centered",
            "social", "ethic"],
    },
    CategorySpec {
        id: "robotics-control", label: "Robotics & Control", tone: "orange",
        description: "Robots, autonomy, sensors and control systems",
        keywords: &["robot", "autonomous", "control system", "simulation", "manipulation",
            "navigation", "sensor", "drone", "motion planning", "actuator"],
    },
    // Fallback only — never selected by scoring, used when nothing else matches.
    CategorySpec {
        id: "other", label: "Other", tone: "",
        description: "Papers that do not match a more specific category",
        keywords: &[],
    },
];

/// Stable taxonomy metadata for API clients and facet labels.
pub fn category_defs() -> Vec<CategoryDef> {
    CATEGORIES.iter().map(|c| CategoryDef {
        id: c.id.into(), label: c.label.into(),
        description: c.description.into(), tone: c.tone.into(),
    }).collect()
}

/// Derive categories from a paper's own fields. Pure and deterministic: the
/// same paper always gets the same result, so the library can be backfilled
/// safely and facets stay stable across restarts.
pub fn categorize(paper: &Paper) -> (Vec<String>, String) {
    let title = paper.title.to_lowercase();

    let mut text = title.clone();
    text.push(' ');
    text.push_str(&paper.abstract_text.to_lowercase());
    text.push(' ');
    text.push_str(&paper.summary.to_lowercase());
    text.push(' ');
    text.push_str(&paper.tags.join(" ").to_lowercase());
    for section in &paper.sections {
        text.push(' ');
        text.push_str(&section.heading.to_lowercase());
    }
    for concept in &paper.concepts {
        text.push(' ');
        text.push_str(&concept.name.to_lowercase());
        text.push(' ');
        text.push_str(&concept.definition.to_lowercase());
    }

    let mut scored: Vec<(i32, &str)> = CATEGORIES.iter()
        .filter(|c| !c.keywords.is_empty())
        .map(|c| {
            let mut score = 0i32;
            for keyword in c.keywords {
                if text.contains(keyword) {
                    score += 1;
                    // A keyword in the title is a much stronger signal.
                    if title.contains(keyword) { score += 2; }
                }
            }
            (score, c.id)
        })
        .collect();

    // Highest score first; break ties by taxonomy order (already stable) via id.
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));

    let mut categories: Vec<String> = scored.into_iter()
        .filter(|(score, _)| *score > 0)
        .take(3)
        .map(|(_, id)| id.to_string())
        .collect();

    if categories.is_empty() {
        categories.push("other".into());
    }
    let primary = categories[0].clone();
    (categories, primary)
}

/// Fill in categories for papers saved before categorisation existed. Returns
/// the number of papers that were updated.
pub fn backfill(papers: &mut [Paper]) -> usize {
    let mut changed = 0;
    for paper in papers.iter_mut() {
        if paper.categories.is_empty() || paper.primary_category.is_empty() {
            let (categories, primary) = categorize(paper);
            paper.categories = categories;
            paper.primary_category = primary;
            changed += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paper_with(title: &str, tags: &[&str], abstract_text: &str) -> Paper {
        let mut p = Paper::default();
        p.title = title.into();
        p.tags = tags.iter().map(|s| s.to_string()).collect();
        p.abstract_text = abstract_text.into();
        p
    }

    #[test]
    fn categorises_security_paper() {
        let p = paper_with(
            "Understanding the Security Boundary of Obfuscation-based On-Device LLM Protection",
            &["security", "TEE", "obfuscation", "model extraction", "systematization"],
            "We systematize obfuscation primitives and demonstrate a model extraction attack against on-device LLM protection using a trusted execution environment.",
        );
        let (cats, primary) = categorize(&p);
        assert_eq!(primary, "security-privacy");
        assert!(cats.contains(&"nlp".to_string()) || cats.contains(&"systems".to_string()));
        assert!(cats.len() <= 3);
    }

    #[test]
    fn categorises_vision_paper() {
        let p = paper_with(
            "Efficient Object Detection in Images",
            &["computer vision", "image", "cnn"],
            "We propose a convolutional neural network for image classification and video object detection with pixel-level segmentation.",
        );
        let (_, primary) = categorize(&p);
        assert_eq!(primary, "vision");
    }

    #[test]
    fn falls_back_to_other() {
        let p = paper_with("A Short Note", &[], "This note has no recognisable topic words at all.");
        let (cats, primary) = categorize(&p);
        assert_eq!(cats, vec!["other"]);
        assert_eq!(primary, "other");
    }

    #[test]
    fn backfill_only_touches_missing() {
        let mut a = paper_with("Security and privacy attacks", &[], "");
        a.primary_category = "custom".into();
        a.categories = vec!["custom".into()];
        let b = paper_with("Security and privacy attacks", &[], "");
        let mut papers = vec![a, b];
        assert_eq!(backfill(&mut papers), 1);
        assert_eq!(papers[0].primary_category, "custom");
        assert!(!papers[1].primary_category.is_empty());
    }

    #[test]
    fn taxonomy_has_unique_ids() {
        let defs = category_defs();
        let mut ids: Vec<&str> = defs.iter().map(|d| d.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), defs.len());
    }
}
