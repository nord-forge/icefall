use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::AppState;

async fn resolve_db_container(state: &AppState, id: &str) -> Result<String, ApiError> {
    let dbs = state.db.list_managed_dbs().await?;
    let db = dbs
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;
    Ok(format!("icefall-db-{}", db.name.to_lowercase()))
}

pub(super) async fn start_database(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let container_name = resolve_db_container(&state, &id).await?;
    state.docker.start_container(&container_name).await?;
    Ok(Json(serde_json::json!({ "message": "started" })))
}

pub(super) async fn stop_database(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let container_name = resolve_db_container(&state, &id).await?;
    state
        .docker
        .stop_container(&container_name, Some(10))
        .await?;
    Ok(Json(serde_json::json!({ "message": "stopped" })))
}

pub(super) async fn restart_database(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let container_name = resolve_db_container(&state, &id).await?;
    state.docker.restart_container(&container_name).await?;
    Ok(Json(serde_json::json!({ "message": "restarted" })))
}
