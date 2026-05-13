use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::api::error::ApiError;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/databases/{id}/backups", get(list_backups))
        .route("/databases/{id}/backup", post(trigger_backup))
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
