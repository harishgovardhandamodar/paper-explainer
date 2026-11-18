pub mod api;
pub mod categories;
pub mod citations;
pub mod corrections;
pub mod error;
pub mod explain;
pub mod exports;
pub mod glossary;
pub mod graph;
pub mod identifiers;
pub mod ingest;
pub mod llm;
pub mod pdf;
pub mod models;
pub mod parse;
pub mod projects;
pub mod provenance;
pub mod questions;
pub mod seed;
pub mod store;
pub mod synthesis;
pub mod tracker;
pub mod util;

use axum::routing::{delete, get, post, put};
use axum::Router;
use std::path::Path;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

pub fn build_router(state: api::ApiState, dist: &Path) -> Router {
    let routes = Router::new()
        .route("/api/health", get(api::health))
        .route("/api/papers", get(api::list_papers).post(api::add_paper))
        .route("/api/resolve", get(api::resolve_identifier))
        .route("/api/library", get(api::library))
        .route("/api/categories", get(api::categories))
        .route("/api/glossary", get(api::glossary))
        .route("/api/glossary/{name}", get(api::glossary_entry))
        .route("/api/papers/{id}", get(api::get_paper).delete(api::delete_paper))
        .route("/api/papers/{id}/refresh", post(api::refresh))
        .route("/api/papers/{id}/enhance", post(api::enhance_paper).delete(api::clear_enhancement))
        .route("/api/papers/{id}/export", get(api::export_markdown))
        .route("/api/papers/{id}/export/{format}", get(api::export_format))
        .route("/api/papers/{id}/open-questions", get(api::open_questions))
        .route("/api/citations", get(api::citations))
        .route("/api/citations/enrich", post(api::enrich_reference))
        .route("/api/papers/{id}/references", get(api::paper_references))
        .route("/api/papers/{id}/references/{ref_id}/enrich", post(api::enrich_paper_reference))
        .route("/api/reading-states", get(api::list_reading_states))
        .route("/api/reading-states/{paper_id}", get(api::get_reading_state).put(api::put_reading_state))
        .route("/api/annotations", get(api::list_annotations).post(api::create_annotation))
        .route("/api/annotations/{id}", put(api::update_annotation).delete(api::delete_annotation))
        .route("/api/followups", get(api::list_follow_ups).post(api::create_follow_up))
        .route("/api/followups/{id}", put(api::update_follow_up).delete(api::delete_follow_up))
        .route("/api/hypothesis-tracker", get(api::hypothesis_tracker))
        .route("/api/syntheses", get(api::list_syntheses).post(api::create_synthesis))
        .route("/api/syntheses/preview", post(api::preview_synthesis))
        .route("/api/syntheses/{id}", get(api::get_synthesis).put(api::update_synthesis).delete(api::delete_synthesis))
        .route("/api/syntheses/{id}/refresh", post(api::refresh_synthesis))
        .route("/api/projects", get(api::list_projects).post(api::create_project))
        .route("/api/projects/{id}", get(api::get_project).put(api::update_project).delete(api::delete_project))
        .route("/api/projects/{id}/export", get(api::project_readout))
        .route("/api/projects/{id}/followups", post(api::add_project_follow_up))
        .route("/api/projects/{id}/followups/{task_id}", put(api::update_project_follow_up).delete(api::delete_project_follow_up))
        .route("/api/provenance", get(api::provenance))
        .route("/api/provenance/backfill", post(api::backfill_provenance))
        .route("/api/provenance/papers/{paper_id}/items/{key}", put(api::update_provenance))
        .route("/api/quality", get(api::quality))
        .route("/api/corrections", get(api::corrections))
        .route("/api/papers/{id}/editable", get(api::editable_items))
        .route("/api/papers/{id}/corrections", post(api::apply_correction))
        .route("/api/papers/{id}/corrections/{correction_id}", delete(api::revert_correction))
        .route("/api/graph", get(api::get_graph))
        .route("/api/graph/mermaid", get(api::get_graph_mermaid))
        .route("/api/graph/edges", post(api::add_edge))
        .route("/api/stats", get(api::stats))
        .route("/api/search", get(api::search))
        .route("/api/reset", post(api::reset))
        .route("/api/llm/status", get(api::llm_status))
        .with_state(state);

    let index = dist.join("index.html");
    let router = if index.exists() {
        // SPA fallback: unknown non-API paths serve index.html.
        routes.fallback_service(ServeDir::new(dist).fallback(ServeFile::new(index)))
    } else {
        routes.fallback(api::not_found)
    };

    router.layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
}
