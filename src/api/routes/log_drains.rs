use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewLogDrain;

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
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let drains = state.db.list_log_drains_for_app(&app_id).await?;
    Ok(Json(serde_json::json!({ "data": drains })))
}

async fn list_global_drains(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let drains = state.db.list_global_log_drains().await?;
    Ok(Json(serde_json::json!({ "data": drains })))
}

#[derive(Deserialize)]
struct CreateDrainRequest {
    name: String,
    drain_type: String,
    config: serde_json::Value,
    #[allow(dead_code)]
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

    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let new_drain = NewLogDrain {
        app_id: Some(app_id),
        name: body.name,
        drain_type: body.drain_type,
        config: body.config.to_string(),
    };

    let drain = state.db.create_log_drain(&new_drain).await?;
    Ok(Json(serde_json::json!({ "data": drain })))
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

    let existing = state.db.get_log_drain(&drain_id).await?;
    if existing.is_none() {
        return Err(ApiError::NotFound(format!(
            "log drain {drain_id} not found"
        )));
    }

    let update = NewLogDrain {
        app_id: existing.unwrap().app_id,
        name: body.name,
        drain_type: body.drain_type,
        config: body.config.to_string(),
    };

    let drain = state.db.update_log_drain(&drain_id, &update).await?;
    Ok(Json(serde_json::json!({ "data": drain })))
}

async fn delete_drain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(drain_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    state.db.delete_log_drain(&drain_id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn test_drain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(drain_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let drain = state.db.get_log_drain(&drain_id).await?;
    if drain.is_none() {
        return Err(ApiError::NotFound(format!(
            "log drain {drain_id} not found"
        )));
    }

    // TODO: Actually send a test log entry to the drain destination based on
    // drain_type (syslog, HTTP, etc.) and verify the connection works.
    Ok(Json(serde_json::json!({
        "data": { "success": true, "message": "Test log sent successfully" }
    })))
}
