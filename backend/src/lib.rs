pub mod api;
pub mod error;
pub mod explain;
pub mod graph;
pub mod ingest;
pub mod llm;
pub mod models;
pub mod parse;
pub mod seed;
pub mod store;
pub mod util;

use axum::routing::{get, post};
use axum::Router;
use std::path::Path;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

pub fn build_router(state: api::ApiState, dist: &Path) -> Router {
    let routes = Router::new()
        .route("/api/health", get(api::health))
        .route("/api/papers", get(api::list_papers).post(api::add_paper))
        .route("/api/papers/{id}", get(api::get_paper).delete(api::delete_paper))
        .route("/api/papers/{id}/refresh", post(api::refresh))
        .route("/api/papers/{id}/enhance", post(api::enhance_paper).delete(api::clear_enhancement))
        .route("/api/papers/{id}/export", get(api::export_markdown))
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
