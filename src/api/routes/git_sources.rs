use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get};
use axum::{Json, Router};

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/git-sources", get(list_sources))
        .route("/git-sources/{id}", delete(delete_source))
        .route("/git-sources/{id}/repos", get(list_repos))
}

async fn list_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "data": [] })))
}

async fn delete_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn list_repos(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "data": [] })))
}
