//! Core domain model for the Paper Explainer.
//!
//! Everything the dashboard renders is expressible in these types. The same
//! model is used for the curated seed paper and for papers ingested from a URL,
//! so the frontend never needs to special-case either path.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Top-level persisted document. One JSON file is our "graph database" for now;
/// the shape maps 1:1 onto tables should this move to SQLite/Neo4j later.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub version: u32,
    pub papers: Vec<Paper>,
    pub graph: KnowledgeGraph,
    /// User-added edges survive graph rebuilds.
    pub custom_edges: Vec<GraphEdge>,
    /// Personal notes and highlights, kept separate from extractor `notes`.
    pub annotations: Vec<Annotation>,
    /// Questions and tasks attached to claims, with triage state.
    pub follow_ups: Vec<FollowUp>,
    /// Stored cross-paper syntheses (related work, comparison, diagram).
    pub syntheses: Vec<Synthesis>,
    /// Per-paper reading triage, keyed by `paper_id` in practice.
    pub reading_states: Vec<ReadingState>,
    /// Named literature-review workspaces with their own notes and tasks.
    pub projects: Vec<Project>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            version: 1, papers: Vec::new(), graph: KnowledgeGraph::default(),
            custom_edges: Vec::new(), annotations: Vec::new(), follow_ups: Vec::new(),
            syntheses: Vec::new(), reading_states: Vec::new(), projects: Vec::new(),
        }
    }

    pub fn find_paper(&self, id: &str) -> Option<&Paper> {
        self.papers.iter().find(|p| p.id == id)
    }

    pub fn find_paper_mut(&mut self, id: &str) -> Option<&mut Paper> {
        self.papers.iter_mut().find(|p| p.id == id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<u32>,
    pub url: String,
    pub venue: Option<String>,
    pub abstract_text: String,
    /// Plain-language overview of the whole paper.
    pub summary: String,
    pub tags: Vec<String>,
    /// Subject categories derived deterministically from the paper text.
    /// Up to three stable taxonomy ids; the first is the primary category.
    pub categories: Vec<String>,
    pub primary_category: String,
    pub status: PaperStatus,
    pub source: PaperSource,
    /// Canonical arXiv id without a version, e.g. `2609.10117`.
    pub arxiv_id: Option<String>,
    /// DOI this paper was ingested from, when available.
    pub doi: Option<String>,
    /// Version suffix from the ingested source, e.g. `v2`.
    pub version: Option<String>,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub hypotheses: Vec<Hypothesis>,
    pub experiments: Vec<Experiment>,
    pub findings: Vec<Finding>,
    pub diagrams: Vec<Diagram>,
    pub workflows: Vec<Workflow>,
    pub concepts: Vec<Concept>,
    pub sections: Vec<PaperSection>,
    /// Bibliography entries parsed from the paper's references section.
    pub references: Vec<PaperReference>,
    /// Non-fatal notes produced by the extractor (e.g. "LLM unavailable").
    pub notes: Vec<String>,
    /// Optional AI-written explanation, generated on demand via Ollama.
    /// Stored alongside the deterministic extraction so it survives reloads.
    pub enhancement: Option<PaperEnhancement>,
    /// Per-item origin, rule and confidence, keyed by item id. Special keys:
    /// `summary` and `enhancement:<field>[:<index>]`.
    #[serde(default)]
    pub item_provenance: std::collections::HashMap<String, Provenance>,
    /// Human corrections to extracted fields. Re-applied after re-import so a
    /// refresh never silently restores a value the reader already fixed.
    #[serde(default)]
    pub corrections: Vec<FieldCorrection>,
}

/// A human edit to one extracted text field, kept as an audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FieldCorrection {
    pub id: String,
    pub paper_id: String,
    /// Item id, or a special key such as `summary`.
    pub item_key: String,
    pub field: String,
    pub original: String,
    pub corrected: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for FieldCorrection {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(), paper_id: String::new(), item_key: String::new(),
            field: String::new(), original: String::new(), corrected: String::new(),
            created_at: now, updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CreateCorrection {
    pub item_key: String,
    pub field: String,
    pub corrected: String,
}

// ---------------------------------------------------------------------------
// Provenance & confidence
// ---------------------------------------------------------------------------

/// The badge shown for an explained item. `Verified` is applied by a human
/// after checking the item against the paper; the original origin is kept in
/// `Provenance::origin` so verification never erases where content came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    Curated,
    Heuristic,
    Ai,
    Manual,
    Verified,
}

impl Default for ProvenanceKind {
    fn default() -> Self { Self::Heuristic }
}

impl ProvenanceKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Curated => "curated",
            Self::Heuristic => "heuristic",
            Self::Ai => "AI",
            Self::Manual => "manual",
            Self::Verified => "verified",
        }
    }
}

/// Provenance for one explained item.
///
/// Confidence is the strength of the extraction signal, **not** a probability
/// that the claim is true. A number-backed finding scores higher than a cue
/// phrase because there is more to check, not because it is more correct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Provenance {
    /// Current badge (origin, or `Verified` after a human check).
    pub kind: ProvenanceKind,
    /// Source before any human verification or manual edit.
    pub origin: ProvenanceKind,
    /// Named deterministic rule or generator, e.g. `finding_numbers`.
    pub rule: String,
    /// Extraction strength in [0, 1]; not a truth probability.
    pub confidence: f32,
    /// Paper section the item was taken from, when the extractor knew it.
    pub section: Option<String>,
    /// Model name or other generator detail, for AI-sourced items.
    pub generator: String,
    pub verified_at: Option<DateTime<Utc>>,
    /// Human note (corrections, caveats, why a badge was changed).
    pub note: String,
    /// True once a human changed a field; protects the entry from re-derivation
    /// on refresh / re-import.
    pub overridden: bool,
    pub updated_at: DateTime<Utc>,
}

impl Default for Provenance {
    fn default() -> Self {
        Self {
            kind: ProvenanceKind::Heuristic,
            origin: ProvenanceKind::Heuristic,
            rule: String::new(),
            confidence: 0.5,
            section: None,
            generator: String::new(),
            verified_at: None,
            note: String::new(),
            overridden: false,
            updated_at: Utc::now(),
        }
    }
}

/// Human edits to one provenance entry. `None` means "leave unchanged".
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateProvenance {
    pub kind: Option<ProvenanceKind>,
    pub rule: Option<String>,
    pub confidence: Option<f32>,
    pub section: Option<String>,
    pub note: Option<String>,
    pub verified: Option<bool>,
}

/// AI-generated explanation layer for a paper. Produced by the configured
/// Ollama model through the fox-services gateway; never replaces the
/// deterministic extraction above.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PaperEnhancement {
    /// One-sentence "what is this paper about" answer.
    pub tldr: String,
    /// Multi-paragraph plain-language explanation of the whole paper.
    pub explanation: String,
    /// Step-by-step explanation of how the work actually works.
    pub how_it_works: String,
    /// The 3–6 things a reader should take away.
    pub key_insights: Vec<String>,
    /// Limitations or caveats the model flagged.
    pub caveats: Vec<String>,
    /// Source anchors and verification state for each discrete AI insight
    /// (key insights and caveats), keyed by their enhancement item key.
    /// Filled deterministically and kept in sync with `key_insights`/`caveats`.
    #[serde(default)]
    pub insight_anchors: Vec<InsightAnchor>,
    /// Model the caller asked for (e.g. qwen3.8:27b).
    pub requested_model: String,
    /// Model that actually answered (the gateway may reroute).
    pub served_model: String,
    /// Ollama base URL used for the call (fox-services gateway).
    pub gateway: String,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub generated_at: DateTime<Utc>,
}

impl Default for PaperEnhancement {
    fn default() -> Self {
        Self {
            tldr: String::new(), explanation: String::new(), how_it_works: String::new(),
            key_insights: Vec::new(), caveats: Vec::new(), insight_anchors: Vec::new(),
            requested_model: String::new(), served_model: String::new(), gateway: String::new(),
            prompt_tokens: None, completion_tokens: None, generated_at: Utc::now(),
        }
    }
}

/// Verification state for one AI-written insight. Anchors start `Unverified`:
/// the app never presents model text as checked until a reader runs the source
/// check or marks it explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightVerification {
    Unverified,
    Supported,
    Contested,
}

impl Default for InsightVerification {
    fn default() -> Self { Self::Unverified }
}

impl InsightVerification {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Supported => "supported",
            Self::Contested => "contested",
        }
    }
}

/// Deterministic source anchor for one AI insight.
///
/// The section and the support score are produced by a lexical check against
/// the paper's text — they are a guardrail signal, not a truth verdict. A human
/// can override the verification state; that override is preserved across
/// regeneration and re-import.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InsightAnchor {
    /// Enhancement item key, e.g. `enhancement:key_insights:0`.
    pub key: String,
    /// Category of the claim: `insight` or `caveat`.
    pub category: String,
    /// Snapshot of the claim text (authoritative copy lives on PaperEnhancement).
    pub text: String,
    /// Paper section the claim is anchored to. Empty means "whole paper".
    pub section: String,
    /// Human verification state, or the result of the automated source check.
    pub verification: InsightVerification,
    /// True once a human explicitly set the verification state.
    pub human: bool,
    /// When the last automated source check ran.
    pub checked_at: Option<DateTime<Utc>>,
    /// Lexical support in [0, 1]: share of the claim's key terms found in the
    /// cited section. Not a probability that the claim is correct.
    pub support: f32,
    /// Claim terms found in the cited section.
    pub matched_terms: Vec<String>,
    /// Claim terms not found in the cited section.
    pub missing_terms: Vec<String>,
    pub note: String,
}

impl Default for InsightAnchor {
    fn default() -> Self {
        Self {
            key: String::new(), category: "insight".into(), text: String::new(),
            section: String::new(), verification: InsightVerification::Unverified,
            human: false, checked_at: None, support: 0.0,
            matched_terms: Vec::new(), missing_terms: Vec::new(), note: String::new(),
        }
    }
}

impl Default for Paper {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(), title: String::new(), authors: Vec::new(), year: None,
            url: String::new(), venue: None, abstract_text: String::new(), summary: String::new(),
            tags: Vec::new(), categories: Vec::new(), primary_category: String::new(),
            status: PaperStatus::Ready, source: PaperSource::Heuristic,
            arxiv_id: None, doi: None, version: None,
            added_at: now, updated_at: now, hypotheses: Vec::new(), experiments: Vec::new(),
            findings: Vec::new(), diagrams: Vec::new(), workflows: Vec::new(),
            concepts: Vec::new(), sections: Vec::new(), references: Vec::new(), notes: Vec::new(),
            enhancement: None, item_provenance: std::collections::HashMap::new(),
            corrections: Vec::new(),
        }
    }
}

impl Paper {
    /// Lightweight card used by the paper list / knowledge graph.
    pub fn to_summary(&self) -> PaperSummary {
        PaperSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            year: self.year,
            url: self.url.clone(),
            summary: self.summary.clone(),
            tags: self.tags.clone(),
            categories: self.categories.clone(),
            primary_category: self.primary_category.clone(),
            status: self.status.clone(),
            source: self.source.clone(),
            arxiv_id: self.arxiv_id.clone(),
            version: self.version.clone(),
            added_at: self.added_at,
            counts: PaperCounts {
                hypotheses: self.hypotheses.len(),
                experiments: self.experiments.len(),
                findings: self.findings.len(),
                diagrams: self.diagrams.len(),
                workflows: self.workflows.len(),
                concepts: self.concepts.len(),
            },
        }
    }

    pub fn all_text(&self) -> String {
        let mut parts = vec![self.title.clone(), self.summary.clone(), self.abstract_text.clone()];
        parts.extend(self.tags.iter().cloned());
        parts.extend(self.categories.iter().cloned());
        parts.extend(self.concepts.iter().map(|c| format!("{} {}", c.name, c.definition)));
        parts.extend(self.hypotheses.iter().map(|h| format!("{} {}", h.statement, h.simplified)));
        parts.extend(self.experiments.iter().map(|e| {
            format!("{} {} {} {}", e.name, e.question, e.design, e.results.join(" "))
        }));
        parts.extend(self.findings.iter().map(|f| format!("{} {}", f.claim, f.evidence)));
        parts.join("\n")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSummary {
    pub id: String,
    pub title: String,
    pub year: Option<u32>,
    pub url: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub primary_category: String,
    pub status: PaperStatus,
    pub source: PaperSource,
    pub arxiv_id: Option<String>,
    pub version: Option<String>,
    pub added_at: DateTime<Utc>,
    pub counts: PaperCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperCounts {
    pub hypotheses: usize,
    pub experiments: usize,
    pub findings: usize,
    pub diagrams: usize,
    pub workflows: usize,
    pub concepts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperStatus {
    Ready,
    Processing,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperSource {
    /// Hand-curated explanation shipped with the app.
    Curated,
    /// Automatically extracted from fetched HTML/text.
    Heuristic,
    /// Automatically extracted with help from an LLM provider.
    LlmAssisted,
    /// Pasted text supplied by the user.
    Manual,
    /// Text extracted from an uploaded PDF.
    Pdf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PaperSection {
    pub heading: String,
    pub level: u8,
    pub paragraphs: Vec<String>,
}

impl Default for PaperSection {
    fn default() -> Self { Self { heading: String::new(), level: 2, paragraphs: Vec::new() } }
}

/// A concept aggregated across the whole collection for the student-friendly
/// glossary. Definitions are kept verbatim; `simple_definition` is a
/// deterministic plain-language rewrite of the same text.
#[derive(Debug, Clone, Serialize)]
pub struct GlossaryEntry {
    pub id: String,
    pub name: String,
    pub definition: String,
    pub simple_definition: String,
    pub category: String,
    pub aliases: Vec<String>,
    /// Names of concepts that a reader should understand first.
    pub prerequisites: Vec<String>,
    /// Papers whose extraction mentions this concept.
    pub paper_ids: Vec<String>,
    pub usage_count: usize,
}

/// One bibliography entry parsed from a paper's reference list. Entries can be
/// resolved to another paper in the collection by canonical arXiv id, DOI or
/// (as a fallback) normalised title, which drives the citation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PaperReference {
    pub id: String,
    /// The raw bibliography line, kept verbatim for provenance.
    pub raw: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<u32>,
    pub arxiv_id: Option<String>,
    pub doi: Option<String>,
    /// Set when this entry matches a paper already in the collection.
    pub resolved_paper_id: Option<String>,
    /// Whether optional Crossref enrichment has been applied to this entry.
    pub enriched: bool,
}

impl Default for PaperReference {
    fn default() -> Self {
        Self {
            id: String::new(), raw: String::new(), title: None, authors: Vec::new(),
            year: None, arxiv_id: None, doi: None, resolved_paper_id: None, enriched: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Hypotheses, experiments, findings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisStatus {
    Supported,
    PartiallySupported,
    Rejected,
    Open,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Hypothesis {
    pub id: String,
    /// The claim as the authors state it (lightly cleaned).
    pub statement: String,
    /// One-paragraph "explain it to me" version.
    pub simplified: String,
    pub status: HypothesisStatus,
    /// IDs of findings that bear on this hypothesis.
    pub evidence_ids: Vec<String>,
    /// IDs of experiments that test it.
    pub experiment_ids: Vec<String>,
    pub section: Option<String>,
    pub confidence: Option<String>,
}

impl Default for Hypothesis {
    fn default() -> Self {
        Self {
            id: String::new(), statement: String::new(), simplified: String::new(),
            status: HypothesisStatus::Open, evidence_ids: Vec::new(), experiment_ids: Vec::new(),
            section: None, confidence: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Experiment {
    pub id: String,
    /// Short label, e.g. "RQ1 · Attack performance across defenses".
    pub name: String,
    /// The question the experiment answers.
    pub question: String,
    /// How the experiment works, in plain language.
    pub design: String,
    pub setup: ExperimentSetup,
    /// Recreated step-by-step procedure.
    pub steps: Vec<WorkflowStep>,
    /// Headline numbers / observations.
    pub results: Vec<String>,
    pub finding_ids: Vec<String>,
    pub hypothesis_ids: Vec<String>,
    pub mermaid: Option<String>,
    pub section: Option<String>,
}

impl Default for Experiment {
    fn default() -> Self {
        Self {
            id: String::new(), name: String::new(), question: String::new(), design: String::new(),
            setup: ExperimentSetup::default(), steps: Vec::new(), results: Vec::new(),
            finding_ids: Vec::new(), hypothesis_ids: Vec::new(), mermaid: None, section: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExperimentSetup {
    pub models: Vec<String>,
    pub datasets: Vec<String>,
    pub metrics: Vec<String>,
    pub baselines: Vec<String>,
    pub conditions: Vec<String>,
    pub budget: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowStep {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

impl Default for WorkflowStep {
    fn default() -> Self {
        Self { id: String::new(), title: String::new(), detail: String::new(), inputs: Vec::new(), outputs: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Finding {
    pub id: String,
    /// Precise claim from the paper.
    pub claim: String,
    /// Simplified restatement for the dashboard.
    pub simplified: String,
    /// Concrete number, quote, or table reference.
    pub evidence: String,
    pub experiment_id: Option<String>,
    pub importance: Importance,
    pub caveat: Option<String>,
}

impl Default for Finding {
    fn default() -> Self {
        Self {
            id: String::new(), claim: String::new(), simplified: String::new(), evidence: String::new(),
            experiment_id: None, importance: Importance::Medium, caveat: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Diagrams & workflows
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramKind {
    Architecture,
    Workflow,
    Taxonomy,
    Timeline,
    Leakage,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Diagram {
    pub id: String,
    pub title: String,
    pub caption: String,
    /// Mermaid source. Rendered client-side.
    pub mermaid: String,
    pub kind: DiagramKind,
    pub paper_id: String,
}

impl Default for Diagram {
    fn default() -> Self {
        Self {
            id: String::new(), title: String::new(), caption: String::new(), mermaid: String::new(),
            kind: DiagramKind::Other, paper_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Workflow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub mermaid: String,
    pub paper_id: String,
    /// "defense" | "attack" | "evaluation" | "general"
    pub category: String,
}

impl Default for Workflow {
    fn default() -> Self {
        Self {
            id: String::new(), title: String::new(), description: String::new(), steps: Vec::new(),
            mermaid: String::new(), paper_id: String::new(), category: "general".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Concepts & knowledge graph
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub definition: String,
    /// e.g. "primitive", "attack", "model", "dataset", "metric"
    pub category: String,
    pub aliases: Vec<String>,
}

impl Default for Concept {
    fn default() -> Self {
        Self { id: String::new(), name: String::new(), definition: String::new(), category: "concept".into(), aliases: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Paper,
    Concept,
    Hypothesis,
    Experiment,
    Finding,
    Method,
    Dataset,
    Metric,
    /// External bibliography entry not (yet) in the collection.
    Reference,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Proposes,
    Tests,
    Supports,
    Challenges,
    Uses,
    Extends,
    Subsumes,
    Refines,
    Produces,
    RelatedTo,
    /// Directed citation: source cites target (paper or reference node).
    Cites,
    /// Inverse of `Cites`: source is cited by target.
    CitedBy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub detail: String,
    pub paper_ids: Vec<String>,
    pub weight: u32,
}

impl Default for GraphNode {
    fn default() -> Self {
        Self { id: String::new(), kind: NodeKind::Concept, label: String::new(), detail: String::new(), paper_ids: Vec::new(), weight: 1 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
    pub label: Option<String>,
    pub paper_ids: Vec<String>,
}

impl Default for GraphEdge {
    fn default() -> Self {
        Self { id: String::new(), source: String::new(), target: String::new(), kind: EdgeKind::RelatedTo, label: None, paper_ids: Vec::new() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

// ---------------------------------------------------------------------------
// Personal annotations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationTarget {
    /// An extracted entity (hypothesis, finding, experiment, concept).
    Entity,
    /// A highlighted range of source text.
    Quote,
    /// A whole section of the paper.
    Section,
}

impl Default for AnnotationTarget {
    fn default() -> Self { Self::Entity }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Annotation {
    pub id: String,
    pub paper_id: String,
    pub target_kind: AnnotationTarget,
    /// Entity id, section heading or a stable quote locator.
    pub target: String,
    /// The text the note is anchored to.
    pub quote: String,
    pub body: String,
    pub tags: Vec<String>,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Annotation {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(), paper_id: String::new(), target_kind: AnnotationTarget::Entity,
            target: String::new(), quote: String::new(), body: String::new(), tags: Vec::new(),
            color: "amber".into(), created_at: now, updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CreateAnnotation {
    pub paper_id: String,
    pub target_kind: AnnotationTarget,
    pub target: String,
    pub quote: String,
    pub body: String,
    pub tags: Vec<String>,
    pub color: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateAnnotation {
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
    pub color: Option<String>,
}

// ---------------------------------------------------------------------------
// Follow-up board
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowUpStatus {
    Open,
    InProgress,
    Done,
}

impl Default for FollowUpStatus {
    fn default() -> Self { Self::Open }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowUpPriority {
    High,
    Medium,
    Low,
}

impl Default for FollowUpPriority {
    fn default() -> Self { Self::Medium }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FollowUp {
    pub id: String,
    pub paper_id: String,
    /// Claim, finding or section this task is attached to.
    pub target: String,
    pub title: String,
    pub notes: String,
    pub status: FollowUpStatus,
    pub priority: FollowUpPriority,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for FollowUp {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(), paper_id: String::new(), target: String::new(), title: String::new(),
            notes: String::new(), status: FollowUpStatus::Open, priority: FollowUpPriority::Medium,
            tags: Vec::new(), created_at: now, updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CreateFollowUp {
    pub paper_id: String,
    pub target: String,
    pub title: String,
    pub notes: String,
    pub status: FollowUpStatus,
    pub priority: FollowUpPriority,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateFollowUp {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub status: Option<FollowUpStatus>,
    pub priority: Option<FollowUpPriority>,
    pub tags: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Literature-review projects
// ---------------------------------------------------------------------------

/// A named workspace grouping papers for one reading project or literature
/// review, with its own notes, follow-up tasks and an exportable readout.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub paper_ids: Vec<String>,
    /// Free-form notes for the project as a whole (markdown encouraged).
    pub notes: String,
    pub tags: Vec<String>,
    pub follow_ups: Vec<ProjectFollowUp>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Project {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(), name: String::new(), description: String::new(),
            paper_ids: Vec::new(), notes: String::new(), tags: Vec::new(),
            follow_ups: Vec::new(), created_at: now, updated_at: now,
        }
    }
}

/// A task scoped to a project rather than to one paper's extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectFollowUp {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub done: bool,
    /// Optional paper within the project that this task refers to.
    pub paper_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Default for ProjectFollowUp {
    fn default() -> Self {
        Self {
            id: String::new(), title: String::new(), notes: String::new(),
            done: false, paper_id: None, created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CreateProject {
    pub name: String,
    pub description: Option<String>,
    pub paper_ids: Vec<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<String>,
    pub paper_ids: Option<Vec<String>>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CreateProjectFollowUp {
    pub title: String,
    pub notes: Option<String>,
    pub paper_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateProjectFollowUp {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub done: Option<bool>,
    pub paper_id: Option<String>,
}

/// Rendered markdown readout for a project.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectReadout {
    pub project_id: String,
    pub title: String,
    pub markdown: String,
}

// ---------------------------------------------------------------------------
// Cross-paper hypothesis tracker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceLink {
    pub id: String,
    pub claim: String,
    pub caveat: Option<String>,
    /// True when the finding qualifies or contradicts the hypothesis.
    pub counter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisEvent {
    pub paper_id: String,
    pub paper_title: String,
    pub paper_year: Option<u32>,
    pub added_at: DateTime<Utc>,
    pub hypothesis_id: String,
    pub statement: String,
    pub simplified: String,
    pub status: HypothesisStatus,
    pub confidence: Option<String>,
    pub section: Option<String>,
    pub evidence: Vec<EvidenceLink>,
    pub counter_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisCluster {
    pub id: String,
    pub label: String,
    pub keywords: Vec<String>,
    /// supported / contested / rejected / open / mixed
    pub verdict: String,
    pub supported: usize,
    pub partial: usize,
    pub open: usize,
    pub rejected: usize,
    pub papers: Vec<String>,
    pub events: Vec<HypothesisEvent>,
    pub counter_points: Vec<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisTracker {
    pub clusters: Vec<HypothesisCluster>,
    /// Every hypothesis event across the corpus, newest paper first.
    pub timeline: Vec<HypothesisEvent>,
    pub total_hypotheses: usize,
    pub papers: usize,
    pub supported: usize,
    pub contested: usize,
    pub open: usize,
}

// ---------------------------------------------------------------------------
// Auto-extracted open questions
// ---------------------------------------------------------------------------

/// A candidate question mined from limitations, future-work, discussion and
/// conclusion text (plus caveats and unresolved claims). Deterministic and
/// reviewable: the UI lets a person edit the text before promoting it to a
/// persisted `FollowUp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenQuestion {
    pub id: String,
    pub paper_id: String,
    /// Where the candidate came from: a section heading, a finding id, etc.
    pub source: String,
    /// The cue that triggered extraction (limitation, future_work, …).
    pub cue: String,
    /// The source sentence the suggestion is based on.
    pub context: String,
    /// Reviewable suggested question text.
    pub text: String,
    /// high / medium / low — strength of the cue, not a truth judgement.
    pub confidence: String,
    /// Suggested anchor entity id (finding / hypothesis), if any.
    pub target: String,
}

impl Default for OpenQuestion {
    fn default() -> Self {
        Self {
            id: String::new(), paper_id: String::new(), source: String::new(),
            cue: String::new(), context: String::new(), text: String::new(),
            confidence: "low".into(), target: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Reading progress & triage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    Unread,
    InProgress,
    Done,
}

impl Default for ReadingStatus {
    fn default() -> Self { Self::Unread }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingPriority {
    High,
    Medium,
    Low,
}

impl Default for ReadingPriority {
    fn default() -> Self { Self::Medium }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadingState {
    pub paper_id: String,
    pub status: ReadingStatus,
    pub priority: ReadingPriority,
    /// Why this paper is worth reading — the triage question.
    pub why: String,
    /// Last dashboard view visited while reading this paper.
    pub last_view: String,
    /// Last entity anchor / position within that view.
    pub last_anchor: String,
    pub updated_at: DateTime<Utc>,
}

impl Default for ReadingState {
    fn default() -> Self {
        Self {
            paper_id: String::new(), status: ReadingStatus::Unread,
            priority: ReadingPriority::Medium, why: String::new(),
            last_view: String::new(), last_anchor: String::new(),
            updated_at: Utc::now(),
        }
    }
}

impl ReadingState {
    pub fn for_paper(paper_id: &str) -> Self {
        Self { paper_id: paper_id.to_string(), ..Self::default() }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateReadingState {
    pub status: Option<ReadingStatus>,
    pub priority: Option<ReadingPriority>,
    pub why: Option<String>,
    pub last_view: Option<String>,
    pub last_anchor: Option<String>,
}

// ---------------------------------------------------------------------------
// Cross-paper synthesis
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisRow {
    /// Comparison dimension (Year, Datasets, Key finding, …).
    pub dimension: String,
    /// One value per paper, aligned with `Synthesis::paper_ids`.
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synthesis {
    pub id: String,
    pub title: String,
    pub paper_ids: Vec<String>,
    pub paper_titles: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Generated related-work narrative (deterministic).
    pub narrative: String,
    /// Generated comparison matrix.
    pub comparison: Vec<SynthesisRow>,
    /// Generated Mermaid diagram linking papers through shared themes.
    pub diagram: String,
    /// Free-form user notes, editable and persisted.
    pub notes: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CreateSynthesis {
    pub paper_ids: Vec<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateSynthesis {
    pub title: Option<String>,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// API request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct IngestRequest {
    /// URL, bare arXiv id or DOI to fetch. Either `url`, `doi`, `pdf` or `text` must be provided.
    pub url: Option<String>,
    /// DOI (with or without a `doi:` prefix or resolver URL).
    pub doi: Option<String>,
    /// Base64-encoded PDF bytes.
    pub pdf: Option<String>,
    /// Original filename for the uploaded PDF, used in the stored URL.
    pub pdf_filename: Option<String>,
    /// Raw paper text/HTML pasted by the user.
    pub text: Option<String>,
    /// Optional human-friendly title override.
    pub title: Option<String>,
    /// Force re-ingest if the paper already exists (e.g. a new arXiv version).
    pub overwrite: bool,
}

impl Default for IngestRequest {
    fn default() -> Self {
        Self {
            url: None, doi: None, pdf: None, pdf_filename: None,
            text: None, title: None, overwrite: false,
        }
    }
}

/// Result of identifying a user-supplied string, before any import happens.
#[derive(Debug, Clone, Serialize)]
pub struct ResolveResponse {
    pub kind: String,
    pub canonical_id: String,
    pub bare_id: Option<String>,
    pub version: Option<String>,
    pub url: Option<String>,
    /// True when an existing paper with this canonical id is in the store.
    pub already_in_library: bool,
    pub existing_title: Option<String>,
}

/// Readiness of the configured Ollama gateway (fox-services).
#[derive(Debug, Clone, Serialize)]
pub struct LlmStatus {
    pub enabled: bool,
    pub reachable: bool,
    pub base_url: String,
    pub model: String,
    pub model_available: bool,
    pub models: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub papers: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsResponse {
    pub papers: usize,
    pub hypotheses: usize,
    pub experiments: usize,
    pub findings: usize,
    pub diagrams: usize,
    pub workflows: usize,
    pub concepts: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub by_status: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub paper_id: String,
    pub paper_title: String,
    /// Primary subject category of the paper this hit belongs to.
    pub paper_category: String,
    pub kind: String,
    pub id: String,
    pub title: String,
    pub snippet: String,
}

// ---------------------------------------------------------------------------
// Categorisation & faceted library retrieval
// ---------------------------------------------------------------------------

/// Stable taxonomy metadata used to render category labels and colours.
#[derive(Debug, Clone, Serialize)]
pub struct CategoryDef {
    pub id: String,
    pub label: String,
    pub description: String,
    pub tone: String,
}

/// One selectable value in a facet, with how many papers carry it.
#[derive(Debug, Clone, Serialize)]
pub struct FacetCount {
    pub value: String,
    pub label: String,
    pub count: usize,
}

/// Counts for each browse facet, computed over the whole collection so the
/// numbers stay meaningful while filters are applied.
#[derive(Debug, Clone, Serialize)]
pub struct LibraryFacets {
    pub categories: Vec<FacetCount>,
    pub tags: Vec<FacetCount>,
    pub sources: Vec<FacetCount>,
    pub statuses: Vec<FacetCount>,
    pub years: Vec<FacetCount>,
    /// Reading-triage facets: unread / in_progress / done.
    pub reading_statuses: Vec<FacetCount>,
    /// Reading-priority facets: high / medium / low.
    pub priorities: Vec<FacetCount>,
}

/// A filtered, sorted, paginated slice of the library plus everything the UI
/// needs to render facets and category labels without extra round-trips.
#[derive(Debug, Clone, Serialize)]
pub struct LibraryResponse {
    pub papers: Vec<PaperSummary>,
    /// Total number of papers matching the current filters.
    pub total: usize,
    /// Number of papers in the collection before filtering.
    pub collection_total: usize,
    pub limit: usize,
    pub offset: usize,
    pub facets: LibraryFacets,
    pub categories: Vec<CategoryDef>,
    /// Reading state for the papers on this page (unread when not tracked).
    pub reading: Vec<ReadingState>,
}

/// Query parameters accepted by `GET /api/library`. All fields are optional so
/// clients can request just a page, just a filter, or the whole catalogue.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LibraryQuery {
    /// Free-text search over title, summary, tags, categories and structure.
    pub q: String,
    /// Category id; empty means all categories.
    pub category: String,
    /// Tag (case-insensitive exact match); empty means all tags.
    pub tag: String,
    /// Source name (curated, heuristic, llm_assisted, manual).
    pub source: String,
    /// Status name (ready, processing, failed).
    pub status: String,
    /// Reading triage status (unread, in_progress, done).
    pub reading_status: String,
    /// Reading priority (high, medium, low).
    pub priority: String,
    pub year_from: Option<u32>,
    pub year_to: Option<u32>,
    /// recent | title | year | relevance | findings
    pub sort: String,
    pub limit: usize,
    pub offset: usize,
}

impl Default for LibraryQuery {
    fn default() -> Self {
        Self {
            q: String::new(), category: String::new(), tag: String::new(),
            source: String::new(), status: String::new(),
            reading_status: String::new(), priority: String::new(),
            year_from: None, year_to: None,
            sort: "recent".into(), limit: 12, offset: 0,
        }
    }
}
