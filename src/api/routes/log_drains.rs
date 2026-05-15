use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps/{id}/log-drains", get(list_drains).post(create_drain))
        .route("/log-drains/{id}", put(update_drain).delete(delete_drain))
        .route("/log-drains/{id}/test", post(test_drain))
        .route("/log-drains", get(list_global_drains))
}

async fn list_drains(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "data": [] })))
}

async fn list_global_drains(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "data": [] })))
}

#[derive(Deserialize)]
struct CreateDrainRequest {
    name: String,
    drain_type: String,
    config: serde_json::Value,
    enabled: Option<bool>,
}

async fn create_drain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(body): Json<CreateDrainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": uuid::Uuid::new_v4().to_string(),
            "app_id": app_id,
            "name": body.name,
            "drain_type": body.drain_type,
            "config": body.config,
            "enabled": body.enabled.unwrap_or(true),
            "last_sent_at": null,
            "created_at": chrono::Utc::now().to_rfc3339(),
        }
    })))
}

async fn update_drain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(drain_id): Path<String>,
    Json(body): Json<CreateDrainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": drain_id,
            "name": body.name,
            "drain_type": body.drain_type,
            "config": body.config,
            "enabled": body.enabled.unwrap_or(true),
        }
    })))
}

async fn delete_drain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_drain_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn test_drain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_drain_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": { "success": true, "message": "Test log sent successfully" }
    })))
}
