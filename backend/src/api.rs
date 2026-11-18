//! HTTP handlers.

use crate::error::{AppError, AppResult};
use crate::graph::graph_to_mermaid;
use crate::ingest::{ingest_paper, refresh_paper};
use crate::llm::LlmClient;
use crate::models::*;
use chrono::Utc;
use crate::store::StoreHandle;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone)]
pub struct ApiState {
    pub store: StoreHandle,
    pub llm: LlmClient,
}

pub type ApiResult<T> = AppResult<T>;

pub async fn health(State(state): State<ApiState>) -> ApiResult<Json<HealthResponse>> {
    let store = state.store.read()?;
    Ok(Json(HealthResponse {
        status: "ok", name: "paper-explainer", version: env!("CARGO_PKG_VERSION"),
        papers: store.papers.len(), graph_nodes: store.graph.nodes.len(), graph_edges: store.graph.edges.len(),
    }))
}

pub async fn list_papers(State(state): State<ApiState>) -> ApiResult<Json<Vec<PaperSummary>>> {
    Ok(Json(state.store.list_papers()?))
}

/// Faceted, filtered, sorted and paginated library retrieval for large corpora.
pub async fn library(
    State(state): State<ApiState>, Query(params): Query<LibraryQuery>,
) -> ApiResult<Json<LibraryResponse>> {
    Ok(Json(state.store.query_library(&params)?))
}

/// Stable category taxonomy metadata (ids, labels, descriptions, tones).
pub async fn categories() -> Json<Vec<CategoryDef>> {
    Json(crate::categories::category_defs())
}

// -- Glossary & concept explorer --------------------------------------------

/// Collection-wide glossary aggregated from every paper's extracted concepts.
pub async fn glossary(State(state): State<ApiState>) -> ApiResult<Json<Vec<GlossaryEntry>>> {
    Ok(Json(crate::glossary::build(&state.store.read()?.papers)))
}

/// One glossary entry looked up by name (case-insensitive) or glossary id.
pub async fn glossary_entry(
    State(state): State<ApiState>, Path(name): Path<String>,
) -> ApiResult<Json<GlossaryEntry>> {
    let store = state.store.read()?;
    let entries = crate::glossary::build(&store.papers);
    let needle = name.to_lowercase();
    entries.into_iter()
        .find(|e| e.name.to_lowercase() == needle || e.id == name
            || e.aliases.iter().any(|a| a.to_lowercase() == needle))
        .ok_or_else(|| AppError::NotFound(format!("glossary term '{name}'")))
        .map(Json)
}

// -- Reading progress & triage ----------------------------------------------

pub async fn list_reading_states(State(state): State<ApiState>) -> ApiResult<Json<Vec<ReadingState>>> {
    Ok(Json(state.store.reading_states()?))
}

pub async fn get_reading_state(
    State(state): State<ApiState>, Path(paper_id): Path<String>,
) -> ApiResult<Json<ReadingState>> {
    Ok(Json(state.store.get_reading_state(&paper_id)?))
}

pub async fn put_reading_state(
    State(state): State<ApiState>, Path(paper_id): Path<String>, Json(req): Json<UpdateReadingState>,
) -> ApiResult<Json<ReadingState>> {
    Ok(Json(state.store.upsert_reading_state(&paper_id, req)?))
}

pub async fn get_paper(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Paper>> {
    Ok(Json(state.store.get_paper(&id)?))
}

// -- Citation graph --------------------------------------------------------

/// Per-paper citation summary (outgoing `cites`, incoming `cited_by`) plus
/// unresolved bibliography counts.
pub async fn citations(State(state): State<ApiState>) -> ApiResult<Json<Vec<crate::citations::CitationEntry>>> {
    Ok(Json(crate::citations::summary(&state.store.read()?.papers)))
}

pub async fn paper_references(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Vec<PaperReference>>> {
    Ok(Json(state.store.get_paper(&id)?.references))
}

#[derive(Debug, Deserialize)]
pub struct EnrichRequest {
    pub doi: String,
}

/// Best-effort Crossref metadata lookup; the caller decides whether to store
/// the returned fields. A failure here never affects the stored extraction.
pub async fn enrich_reference(Json(req): Json<EnrichRequest>) -> ApiResult<Json<PaperReference>> {
    Ok(Json(crate::citations::enrich_crossref(&req.doi).await?))
}

/// Enrich a stored bibliography entry in place with Crossref metadata.
pub async fn enrich_paper_reference(
    State(state): State<ApiState>, Path((id, ref_id)): Path<(String, String)>,
) -> ApiResult<Json<PaperReference>> {
    let mut paper = state.store.get_paper(&id)?;
    let doi = paper.references.iter().find(|r| r.id == ref_id)
        .and_then(|r| r.doi.clone())
        .ok_or_else(|| AppError::NotFound(format!("reference '{ref_id}' (or its DOI)")))?;
    let enriched = crate::citations::enrich_crossref(&doi).await?;
    let target = paper.references.iter_mut().find(|r| r.id == ref_id).unwrap();
    if enriched.title.is_some() { target.title = enriched.title; }
    if !enriched.authors.is_empty() { target.authors = enriched.authors; }
    if enriched.year.is_some() { target.year = enriched.year; }
    if enriched.arxiv_id.is_some() { target.arxiv_id = enriched.arxiv_id; }
    target.enriched = true;
    let result = target.clone();
    state.store.upsert_paper(paper)?;
    Ok(Json(result))
}

pub async fn open_questions(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Vec<OpenQuestion>>> {
    let paper = state.store.get_paper(&id)?;
    Ok(Json(crate::questions::extract(&paper)))
}

pub async fn add_paper(
    State(state): State<ApiState>, Json(req): Json<IngestRequest>,
) -> ApiResult<(StatusCode, Json<Paper>)> {
    let paper = ingest_paper(req, &state.store).await?;
    Ok((StatusCode::CREATED, Json(paper)))
}

#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    pub value: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ProvenanceQuery {
    /// Restrict the report to one paper.
    pub paper_id: Option<String>,
}

/// Identify an arXiv id / DOI before importing, so the UI can warn about a
/// duplicate (including version differences) without changing any state.
pub async fn resolve_identifier(
    State(state): State<ApiState>, Query(params): Query<ResolveQuery>,
) -> ApiResult<Json<ResolveResponse>> {
    let resolved = crate::identifiers::parse(&params.value);
    let existing_title = {
        let store = state.store.read()?;
        if resolved.canonical_id.is_empty() { None }
        else { store.find_paper(&resolved.canonical_id).map(|p| p.title.clone()) }
    };
    Ok(Json(ResolveResponse {
        kind: match resolved.kind {
            crate::identifiers::IdentifierKind::Arxiv => "arxiv",
            crate::identifiers::IdentifierKind::Doi => "doi",
            crate::identifiers::IdentifierKind::Other => "other",
        }.into(),
        canonical_id: resolved.canonical_id,
        bare_id: resolved.bare_id,
        version: resolved.version,
        url: resolved.url,
        already_in_library: existing_title.is_some(),
        existing_title,
    }))
}

pub async fn delete_paper(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.store.delete_paper(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn refresh(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Paper>> {
    Ok(Json(refresh_paper(&id, &state.store).await?))
}

/// Readiness of the Ollama gateway used for AI enhancements.
pub async fn llm_status(State(state): State<ApiState>) -> Json<LlmStatus> {
    Json(state.llm.status().await)
}

/// Generate (or regenerate) the AI explanation for one paper. The result is
/// persisted on the paper so it survives reloads and graph rebuilds.
pub async fn enhance_paper(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Paper>> {
    let paper = state.store.get_paper(&id)?;
    let enhancement = state.llm.enhance(&paper).await?;
    let updated = {
        let mut store = state.store.write()?;
        let p = store.find_paper_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("paper '{id}'")))?;
        p.enhancement = Some(enhancement);
        p.updated_at = Utc::now();
        crate::provenance::refresh_enhancement_provenance(p);
        p.clone()
    };
    state.store.flush()?;
    Ok(Json(updated))
}

/// Remove a stored AI explanation, returning to the deterministic view only.
pub async fn clear_enhancement(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Paper>> {
    let updated = {
        let mut store = state.store.write()?;
        let p = store.find_paper_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("paper '{id}'")))?;
        p.enhancement = None;
        p.updated_at = Utc::now();
        crate::provenance::refresh_enhancement_provenance(p);
        p.clone()
    };
    state.store.flush()?;
    Ok(Json(updated))
}

pub async fn get_graph(State(state): State<ApiState>) -> ApiResult<Json<KnowledgeGraph>> {
    Ok(Json(state.store.graph()?))
}

pub async fn get_graph_mermaid(State(state): State<ApiState>) -> ApiResult<String> {
    Ok(graph_to_mermaid(&state.store.graph()?))
}

pub async fn add_edge(
    State(state): State<ApiState>, Json(mut edge): Json<GraphEdge>,
) -> ApiResult<(StatusCode, Json<GraphEdge>)> {
    if edge.id.is_empty() {
        edge.id = format!("custom|{}|{}|{}", edge.source, edge.target, serde_json::to_string(&edge.kind)?);
    }
    let edge = state.store.add_edge(edge)?;
    Ok((StatusCode::CREATED, Json(edge)))
}

pub async fn stats(State(state): State<ApiState>) -> ApiResult<Json<StatsResponse>> {
    let store = state.store.read()?;
    Ok(Json(StatsResponse {
        papers: store.papers.len(),
        hypotheses: store.papers.iter().map(|p| p.hypotheses.len()).sum(),
        experiments: store.papers.iter().map(|p| p.experiments.len()).sum(),
        findings: store.papers.iter().map(|p| p.findings.len()).sum(),
        diagrams: store.papers.iter().map(|p| p.diagrams.len()).sum(),
        workflows: store.papers.iter().map(|p| p.workflows.len()).sum(),
        concepts: store.papers.iter().map(|p| p.concepts.len()).sum(),
        graph_nodes: store.graph.nodes.len(),
        graph_edges: store.graph.edges.len(),
        by_status: json!({
            "ready": store.papers.iter().filter(|p| p.status == PaperStatus::Ready).count(),
            "processing": store.papers.iter().filter(|p| p.status == PaperStatus::Processing).count(),
            "failed": store.papers.iter().filter(|p| p.status == PaperStatus::Failed).count(),
        }),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
fn default_limit() -> usize { 20 }

pub async fn search(
    State(state): State<ApiState>, Query(params): Query<SearchQuery>,
) -> ApiResult<Json<Vec<SearchResult>>> {
    let needle = params.q.trim().to_lowercase();
    if needle.is_empty() { return Ok(Json(Vec::new())); }
    let store = state.store.read()?;
    let mut out = Vec::new();
    for p in &store.papers {
        let mut push = |kind: &str, id: String, title: String, body: String| {
            if body.to_lowercase().contains(&needle) || title.to_lowercase().contains(&needle) {
                out.push(SearchResult {
                    paper_id: p.id.clone(), paper_title: p.title.clone(),
                    paper_category: p.primary_category.clone(), kind: kind.into(),
                    id, title, snippet: truncate_snippet(&body, &needle, 180),
                });
            }
        };
        push("paper", p.id.clone(), p.title.clone(), format!("{} {}", p.summary, p.abstract_text));
        for h in &p.hypotheses { push("hypothesis", h.id.clone(), h.statement.clone(), h.simplified.clone()); }
        for e in &p.experiments { push("experiment", e.id.clone(), e.name.clone(), format!("{} {}", e.question, e.design)); }
        for f in &p.findings { push("finding", f.id.clone(), f.claim.clone(), format!("{} {}", f.simplified, f.evidence)); }
        for c in &p.concepts { push("concept", c.id.clone(), c.name.clone(), c.definition.clone()); }
    }
    out.truncate(params.limit.min(100));
    Ok(Json(out))
}

fn truncate_snippet(body: &str, needle: &str, max: usize) -> String {
    let lower = body.to_lowercase();
    let pos = lower.find(needle).unwrap_or(0);
    let start = pos.saturating_sub(50);
    let end = (start + max).min(body.len());
    let mut s = body.get(start..end).unwrap_or(body).trim().to_string();
    if start > 0 { s = format!("…{s}"); }
    if end < body.len() { s.push('…'); }
    s
}

pub async fn export_markdown(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Response> {
    let p = state.store.get_paper(&id)?;
    let mut md = format!("# {}\n\n", p.title);
    md.push_str(&format!("**Authors:** {}  \n", p.authors.join(", ")));
    md.push_str(&format!("**Source:** [{}]({})  \n", p.venue.as_deref().unwrap_or("paper"), p.url));
    md.push_str("\n## Plain-language summary\n\n");
    md.push_str(&p.summary);
    if let Some(e) = &p.enhancement {
        md.push_str("\n\n## AI-enhanced explanation\n\n");
        md.push_str(&format!("> {}\n\n", e.tldr));
        md.push_str(&e.explanation);
        md.push_str("\n\n### How it works\n\n");
        md.push_str(&e.how_it_works);
        md.push_str("\n\n### Key insights\n\n");
        for x in &e.key_insights { md.push_str(&format!("- {x}\n")); }
        if !e.caveats.is_empty() {
            md.push_str("\n### Caveats\n\n");
            for x in &e.caveats { md.push_str(&format!("- {x}\n")); }
        }
        md.push_str(&format!(
            "\n\n*AI-generated via {} — requested {}, served {}. Verify against the paper.*\n",
            e.gateway, e.requested_model, e.served_model
        ));
    }
    md.push_str("\n\n## Hypotheses\n\n");
    for h in &p.hypotheses {
        md.push_str(&format!("### {} — {:?}\n\n{}\n\n> {}\n\n", h.id, h.status, h.statement, h.simplified));
    }
    md.push_str("## Experiments\n\n");
    for e in &p.experiments {
        md.push_str(&format!("### {}\n\n**Question:** {}\n\n{}\n\n", e.name, e.question, e.design));
        for r in &e.results { md.push_str(&format!("- {}\n", r)); }
        md.push('\n');
    }
    md.push_str("## Key findings\n\n");
    for f in &p.findings {
        md.push_str(&format!("### {}\n\n{}\n\n> {}\n\n**Evidence:** {}\n\n", f.id, f.claim, f.simplified, f.evidence));
        if let Some(c) = &f.caveat { md.push_str(&format!("*Caveat:* {}\n\n", c)); }
    }
    md.push_str("## Diagrams (Mermaid)\n\n");
    for d in &p.diagrams {
        md.push_str(&format!("### {}\n\n{}\n\n```mermaid\n{}\n```\n\n", d.title, d.caption, d.mermaid));
    }
    md.push_str("## Workflows\n\n");
    for w in &p.workflows {
        md.push_str(&format!("### {}\n\n{}\n\n", w.title, w.description));
        for (i, s) in w.steps.iter().enumerate() {
            md.push_str(&format!("{}. **{}** — {}\n", i + 1, s.title, s.detail));
        }
        md.push('\n');
    }
    let headers = [
        (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
        (header::CONTENT_DISPOSITION, "attachment"),
    ];
    let mut resp = md.into_response();
    for (k, v) in headers { resp.headers_mut().insert(k, v.parse().unwrap()); }
    Ok(resp)
}

/// Additional export formats: bibtex, findings.csv, findings.json, deck.mmd, brief.html.
pub async fn export_format(
    State(state): State<ApiState>, Path((id, format)): Path<(String, String)>,
) -> ApiResult<Response> {
    let p = state.store.get_paper(&id)?;
    let (mime, body, inline) = match format.as_str() {
        "bibtex" => ("application/x-bibtex; charset=utf-8", crate::exports::bibtex(&p), false),
        "findings.csv" => ("text/csv; charset=utf-8", crate::exports::findings_csv(&p), false),
        "findings.json" => ("application/json; charset=utf-8", crate::exports::findings_json(&p), false),
        "deck.mmd" => ("text/markdown; charset=utf-8", crate::exports::deck(&p), false),
        "brief.html" => ("text/html; charset=utf-8", crate::exports::brief_html(&p), true),
        _ => return Err(AppError::BadRequest(format!("unsupported export format '{format}'"))),
    };
    let disposition = if inline { "inline" } else { "attachment" };
    let mut resp = body.into_response();
    let headers = [
        (header::CONTENT_TYPE, mime),
        (header::CONTENT_DISPOSITION, disposition),
    ];
    for (k, v) in headers { resp.headers_mut().insert(k, v.parse().unwrap()); }
    Ok(resp)
}

#[derive(Deserialize, Default)]
pub struct AnnotationQuery {
    pub paper_id: Option<String>,
}

pub async fn list_annotations(
    State(state): State<ApiState>, Query(q): Query<AnnotationQuery>,
) -> ApiResult<Json<Vec<Annotation>>> {
    Ok(Json(state.store.annotations(q.paper_id.as_deref())?))
}

pub async fn create_annotation(
    State(state): State<ApiState>, Json(req): Json<CreateAnnotation>,
) -> ApiResult<Json<Annotation>> {
    Ok(Json(state.store.add_annotation(req)?))
}

pub async fn update_annotation(
    State(state): State<ApiState>, Path(id): Path<String>, Json(req): Json<UpdateAnnotation>,
) -> ApiResult<Json<Annotation>> {
    Ok(Json(state.store.update_annotation(&id, req)?))
}

pub async fn delete_annotation(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.store.delete_annotation(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_follow_ups(
    State(state): State<ApiState>, Query(q): Query<AnnotationQuery>,
) -> ApiResult<Json<Vec<FollowUp>>> {
    Ok(Json(state.store.follow_ups(q.paper_id.as_deref())?))
}

/// All hypotheses in the corpus, clustered into shared claims with status
/// history and counter-evidence. Read-only and derived on request.
pub async fn hypothesis_tracker(State(state): State<ApiState>) -> ApiResult<Json<HypothesisTracker>> {
    let store = state.store.read()?;
    Ok(Json(crate::tracker::build_tracker(&store.papers)))
}

// -- Cross-paper syntheses ---------------------------------------------------

pub async fn list_syntheses(State(state): State<ApiState>) -> ApiResult<Json<Vec<Synthesis>>> {
    Ok(Json(state.store.syntheses()?))
}

pub async fn get_synthesis(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Synthesis>> {
    Ok(Json(state.store.get_synthesis(&id)?))
}

pub async fn preview_synthesis(
    State(state): State<ApiState>, Json(req): Json<CreateSynthesis>,
) -> ApiResult<Json<Synthesis>> {
    Ok(Json(state.store.preview_synthesis(req)?))
}

pub async fn create_synthesis(
    State(state): State<ApiState>, Json(req): Json<CreateSynthesis>,
) -> ApiResult<(StatusCode, Json<Synthesis>)> {
    Ok((StatusCode::CREATED, Json(state.store.add_synthesis(req)?)))
}

pub async fn update_synthesis(
    State(state): State<ApiState>, Path(id): Path<String>, Json(req): Json<UpdateSynthesis>,
) -> ApiResult<Json<Synthesis>> {
    Ok(Json(state.store.update_synthesis(&id, req)?))
}

pub async fn refresh_synthesis(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Synthesis>> {
    Ok(Json(state.store.refresh_synthesis(&id)?))
}

pub async fn delete_synthesis(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.store.delete_synthesis(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_follow_up(
    State(state): State<ApiState>, Json(req): Json<CreateFollowUp>,
) -> ApiResult<Json<FollowUp>> {
    Ok(Json(state.store.add_follow_up(req)?))
}

pub async fn update_follow_up(
    State(state): State<ApiState>, Path(id): Path<String>, Json(req): Json<UpdateFollowUp>,
) -> ApiResult<Json<FollowUp>> {
    Ok(Json(state.store.update_follow_up(&id, req)?))
}

pub async fn delete_follow_up(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.store.delete_follow_up(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

// -- Literature-review projects ---------------------------------------------

pub async fn list_projects(State(state): State<ApiState>) -> ApiResult<Json<Vec<Project>>> {
    Ok(Json(state.store.projects()?))
}

pub async fn get_project(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Project>> {
    Ok(Json(state.store.get_project(&id)?))
}

pub async fn create_project(
    State(state): State<ApiState>, Json(req): Json<CreateProject>,
) -> ApiResult<(StatusCode, Json<Project>)> {
    Ok((StatusCode::CREATED, Json(state.store.create_project(req)?)))
}

pub async fn update_project(
    State(state): State<ApiState>, Path(id): Path<String>, Json(req): Json<UpdateProject>,
) -> ApiResult<Json<Project>> {
    Ok(Json(state.store.update_project(&id, req)?))
}

pub async fn delete_project(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.store.delete_project(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn project_readout(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<ProjectReadout>> {
    Ok(Json(state.store.project_readout(&id)?))
}

pub async fn add_project_follow_up(
    State(state): State<ApiState>, Path(id): Path<String>, Json(req): Json<CreateProjectFollowUp>,
) -> ApiResult<Json<ProjectFollowUp>> {
    Ok(Json(state.store.add_project_follow_up(&id, req)?))
}

pub async fn update_project_follow_up(
    State(state): State<ApiState>, Path((id, task_id)): Path<(String, String)>,
    Json(req): Json<UpdateProjectFollowUp>,
) -> ApiResult<Json<ProjectFollowUp>> {
    Ok(Json(state.store.update_project_follow_up(&id, &task_id, req)?))
}

pub async fn delete_project_follow_up(
    State(state): State<ApiState>, Path((id, task_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    state.store.delete_project_follow_up(&id, &task_id)?;
    Ok(StatusCode::NO_CONTENT)
}

// -- Provenance & confidence -------------------------------------------------

/// Every explained item in the collection with its badge, rule and confidence.
pub async fn provenance(
    State(state): State<ApiState>, Query(params): Query<ProvenanceQuery>,
) -> ApiResult<Json<crate::provenance::ProvenanceReport>> {
    Ok(Json(state.store.provenance_report(params.paper_id.as_deref())?))
}

/// Update one item's badge, rule, confidence, note or verification state.
pub async fn update_provenance(
    State(state): State<ApiState>,
    Path((paper_id, key)): Path<(String, String)>,
    Json(req): Json<UpdateProvenance>,
) -> ApiResult<Json<Provenance>> {
    Ok(Json(state.store.update_provenance(&paper_id, &key, req)?))
}

/// Re-derive missing provenance after a schema upgrade or bulk import.
pub async fn backfill_provenance(State(state): State<ApiState>) -> ApiResult<Json<serde_json::Value>> {
    let added = state.store.backfill_provenance()?;
    Ok(Json(json!({ "added": added })))
}

// -- Corrections & extraction quality ----------------------------------------

/// Optional paper filter for correction listings.
#[derive(Debug, Default, Deserialize)]
pub struct CorrectionQuery {
    pub paper_id: Option<String>,
}

/// Editable items and fields for one paper (from the backend whitelist).
pub async fn editable_items(
    State(state): State<ApiState>, Path(id): Path<String>,
) -> ApiResult<Json<Vec<crate::corrections::EditableItem>>> {
    Ok(Json(state.store.editable_items(&id)?))
}

pub async fn corrections(
    State(state): State<ApiState>, Query(params): Query<CorrectionQuery>,
) -> ApiResult<Json<Vec<crate::corrections::CorrectionView>>> {
    Ok(Json(state.store.corrections(params.paper_id.as_deref())?))
}

pub async fn apply_correction(
    State(state): State<ApiState>, Path(id): Path<String>, Json(req): Json<CreateCorrection>,
) -> ApiResult<Json<FieldCorrection>> {
    Ok(Json(state.store.apply_correction(&id, req)?))
}

pub async fn revert_correction(
    State(state): State<ApiState>, Path((id, correction_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    state.store.revert_correction(&id, &correction_id)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Dashboard joining extraction rules to human corrections and confidence.
pub async fn quality(State(state): State<ApiState>) -> ApiResult<Json<crate::corrections::QualityReport>> {
    Ok(Json(state.store.quality_report()?))
}

pub async fn reset(State(state): State<ApiState>) -> ApiResult<StatusCode> {
    state.store.reset()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": { "code": "not_found", "message": "route not found" } }))).into_response()
}

#[allow(dead_code)]
fn _unused(_: &AppError) {}
