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
        .route(
            "/databases/{id}/backups/{backup_id}/restore",
            post(restore_backup),
        )
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

    if !crate::monitoring::backup_scheduler::backup_supported(&db.db_type) {
        return Err(ApiError::BadRequest(format!(
            "Backups are not supported for {} databases",
            db.db_type
        )));
    }

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

/// The shell command that restores a gzipped dump staged at `/tmp/restore_dump.gz`.
/// Mirrors the dump commands in the backup scheduler. Returns `None` for engines
/// whose backup format can't be restored through a simple client pipe.
fn restore_command(db_type: &str) -> Option<&'static str> {
    match db_type {
        "postgres" => Some("gunzip -c /tmp/restore_dump.gz | psql -U icefall -d postgres 2>&1"),
        "mysql" => Some("gunzip -c /tmp/restore_dump.gz | mysql -u icefall 2>&1"),
        "mariadb" => Some("gunzip -c /tmp/restore_dump.gz | mariadb -u icefall 2>&1"),
        "mongo" => Some(
            "mongorestore --archive=/tmp/restore_dump.gz --gzip --username icefall --drop 2>&1",
        ),
        _ => None,
    }
}

async fn restore_backup(
    State(state): State<AppState>,
    Path((id, backup_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Reject path-traversal in the backup id before it touches the filesystem.
    if backup_id.contains('/') || backup_id.contains('\\') || backup_id.contains("..") {
        return Err(ApiError::BadRequest("invalid backup id".into()));
    }

    let dbs = state.db.list_managed_dbs().await?;
    let db = dbs
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;

    let restore_cmd = restore_command(&db.db_type).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "Restoring a backup is not supported for {} databases",
            db.db_type
        ))
    })?;

    let path = state
        .backup_store
        .get_backup_path(&id, &backup_id)
        .ok_or_else(|| ApiError::NotFound(format!("backup {backup_id}")))?;
    let bytes = std::fs::read(&path).map_err(ApiError::internal)?;

    let container_name = format!("icefall-db-{}", db.name.to_lowercase());

    // Stage the dump inside the container, then run the engine restore.
    state
        .docker
        .write_file_to_container(&container_name, "/tmp/restore_dump.gz", &bytes)
        .await
        .map_err(ApiError::internal)?;

    let cmd = vec!["sh".to_string(), "-c".to_string(), restore_cmd.to_string()];
    let output = state
        .docker
        .exec_in_container(&container_name, &cmd)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(serde_json::json!({
        "message": "Backup restored",
        "output": output,
    })))
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
