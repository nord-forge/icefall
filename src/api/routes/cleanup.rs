use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cleanup-schedule", get(get_schedule).put(update_schedule))
        .route("/cleanup-schedule/run", post(run_cleanup))
        .route("/cleanup-schedule/history", get(list_history))
}

async fn get_schedule(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "cron": "0 2 * * 0",
            "disk_threshold_percent": 80,
            "dangling_images": true,
            "unused_images": false,
            "stopped_containers": true,
            "stopped_container_age_hours": 48,
            "volumes": false,
            "networks": false,
            "enabled": false,
        }
    })))
}

#[derive(Deserialize)]
struct UpdateScheduleRequest {
    cron: Option<String>,
    disk_threshold_percent: Option<u8>,
    dangling_images: Option<bool>,
    unused_images: Option<bool>,
    stopped_containers: Option<bool>,
    stopped_container_age_hours: Option<u32>,
    volumes: Option<bool>,
    networks: Option<bool>,
    enabled: Option<bool>,
}

async fn update_schedule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateScheduleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "cron": body.cron.unwrap_or_else(|| "0 2 * * 0".into()),
            "disk_threshold_percent": body.disk_threshold_percent.unwrap_or(80),
            "dangling_images": body.dangling_images.unwrap_or(true),
            "unused_images": body.unused_images.unwrap_or(false),
            "stopped_containers": body.stopped_containers.unwrap_or(true),
            "stopped_container_age_hours": body.stopped_container_age_hours.unwrap_or(48),
            "volumes": body.volumes.unwrap_or(false),
            "networks": body.networks.unwrap_or(false),
            "enabled": body.enabled.unwrap_or(false),
        },
        "message": "Schedule updated",
    })))
}

async fn run_cleanup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "status": "running",
            "message": "Cleanup started",
        }
    })))
}

async fn list_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "data": [] })))
}
