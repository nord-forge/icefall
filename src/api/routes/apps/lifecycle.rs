use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::AppState;

pub(super) async fn start_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;

    let mut started = 0u32;
    for container in &containers {
        if container.state != "running" {
            state.docker.start_container(&container.id).await?;
            started += 1;
        }
    }

    Ok(Json(
        serde_json::json!({ "message": "started", "containers": started }),
    ))
}

pub(super) async fn stop_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;

    let mut stopped = 0u32;
    for container in &containers {
        if container.state == "running" {
            state.docker.stop_container(&container.id, Some(10)).await?;
            stopped += 1;
        }
    }

    Ok(Json(
        serde_json::json!({ "message": "stopped", "containers": stopped }),
    ))
}

pub(super) async fn restart_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;

    let mut restarted = 0u32;
    for container in &containers {
        if container.state == "running" {
            state.docker.restart_container(&container.id).await?;
            restarted += 1;
        }
    }

    Ok(Json(
        serde_json::json!({ "message": "restarted", "containers": restarted }),
    ))
}
