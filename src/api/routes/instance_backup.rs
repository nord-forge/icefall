use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/settings/instance-backup",
            get(get_config).put(update_config),
        )
        .route("/settings/instance-backup/trigger", post(trigger_backup))
        .route("/settings/instance-backup/history", get(list_history))
}

async fn get_config(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let config = state.db.get_instance_backup_config().await?;
    match config {
        Some(c) => Ok(Json(serde_json::json!({ "data": c }))),
        None => Ok(Json(serde_json::json!({
            "data": {
                "enabled": false,
                "cron_schedule": "daily",
                "retention_count": 7,
            }
        }))),
    }
}

#[derive(Deserialize)]
struct UpdateConfigRequest {
    enabled: Option<bool>,
    cron_schedule: Option<String>,
    retention_count: Option<i64>,
}

async fn update_config(
    State(state): State<AppState>,
    Json(body): Json<UpdateConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get current config to merge with
    let current = state.db.get_instance_backup_config().await?;
    let enabled = body
        .enabled
        .unwrap_or_else(|| current.as_ref().is_some_and(|c| c.enabled));
    let cron_schedule = body.cron_schedule.unwrap_or_else(|| {
        current
            .as_ref().map_or_else(|| "daily".to_string(), |c| c.cron_schedule.clone())
    });
    let retention_count = body
        .retention_count
        .unwrap_or_else(|| current.as_ref().map_or(7, |c| c.retention_count));

    // Validate schedule
    if !["daily", "weekly", "monthly"].contains(&cron_schedule.as_str()) {
        return Err(ApiError::BadRequest(
            "cron_schedule must be one of: daily, weekly, monthly".into(),
        ));
    }

    if !(1..=365).contains(&retention_count) {
        return Err(ApiError::BadRequest(
            "retention_count must be between 1 and 365".into(),
        ));
    }

    let config = state
        .db
        .upsert_instance_backup_config(enabled, &cron_schedule, retention_count)
        .await?;

    Ok(Json(serde_json::json!({ "data": config })))
}

async fn trigger_backup(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = state.instance_backup_handle.clone();

    // Spawn the backup in a background task so we can return immediately
    tokio::spawn(async move {
        match handle.trigger().await {
            Ok(id) => tracing::info!("Manual instance backup completed: {id}"),
            Err(e) => tracing::error!("Manual instance backup failed: {e}"),
        }
    });

    Ok(Json(serde_json::json!({
        "message": "Instance backup triggered",
        "status": "running"
    })))
}

async fn list_history(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let history = state.db.list_instance_backup_history(50).await?;
    Ok(Json(serde_json::json!({ "data": history })))
}
