use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};

use crate::api::error::ApiError;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/databases/{id}/backups", get(list_backups))
        .route("/databases/{id}/backup", post(trigger_backup))
        .route("/databases/{id}/backups/{backup_id}", delete(delete_backup))
}

async fn list_backups(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let backups = state.backup_store.list_backups(&id);
    Ok(Json(serde_json::json!({ "data": backups })))
}

async fn trigger_backup(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dbs = state.db.list_managed_dbs().await?;
    let db = dbs
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;

    let container_name = format!("icefall-db-{}", db.name.to_lowercase());

    match state
        .backup_store
        .run_backup(&state.docker, &db.id, &db.db_type, &container_name)
        .await
    {
        Ok(info) => Ok(Json(serde_json::json!({ "data": info }))),
        Err(e) => Err(ApiError::internal(std::io::Error::other(e))),
    }
}

async fn delete_backup(
    State(state): State<AppState>,
    Path((id, backup_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Reject path-traversal in the backup id before it reaches the filesystem.
    if backup_id.contains('/') || backup_id.contains('\\') || backup_id.contains("..") {
        return Err(ApiError::BadRequest("invalid backup id".into()));
    }

    // The database must exist (and the file lives under its own backup dir).
    let dbs = state.db.list_managed_dbs().await?;
    if !dbs.iter().any(|d| d.id == id) {
        return Err(ApiError::NotFound(format!("database {id}")));
    }

    match state.backup_store.delete_backup(&id, &backup_id) {
        Ok(true) => Ok(Json(serde_json::json!({ "message": "deleted" }))),
        Ok(false) => Err(ApiError::NotFound(format!("backup {backup_id}"))),
        Err(e) => Err(ApiError::internal(e)),
    }
}
