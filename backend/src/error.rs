use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("fetch failed: {0}")]
    Fetch(String),
    #[error("parse failed: {0}")]
    Parse(String),
    #[error("llm gateway error: {0}")]
    Llm(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, "not_found", m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
            AppError::Fetch(m) => (StatusCode::BAD_GATEWAY, "fetch_failed", m.clone()),
            AppError::Parse(m) => (StatusCode::UNPROCESSABLE_ENTITY, "parse_failed", m.clone()),
            AppError::Llm(m) => (StatusCode::BAD_GATEWAY, "llm_error", m.clone()),
            AppError::Io(m) => (StatusCode::INTERNAL_SERVER_ERROR, "io_error", m.to_string()),
            AppError::Json(m) => (StatusCode::INTERNAL_SERVER_ERROR, "json_error", m.to_string()),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", m.clone()),
        };
        tracing::warn!(status = %status, code, message, "request failed");
        (status, Json(json!({ "error": { "code": code, "message": message } }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
