use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;

/// Resolve a managed DB's container name, but only if the database
/// belongs to the caller's team with at least member role.
async fn resolve_db_container(
    state: &AppState,
    ctx: &TeamCtx,
    id: &str,
) -> Result<String, ApiError> {
    let db = state
        .db
        .get_managed_db_for_team(&ctx.team_id, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;
    ctx.verify_team_access(&db.team_id, TeamRole::Member)?;
    Ok(format!("icefall-db-{}", db.name.to_lowercase()))
}

pub(super) async fn start_database(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let container_name = resolve_db_container(&state, &ctx, &id).await?;
    state.docker.start_container(&container_name).await?;
    Ok(Json(serde_json::json!({ "message": "started" })))
}

pub(super) async fn stop_database(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let container_name = resolve_db_container(&state, &ctx, &id).await?;
    state
        .docker
        .stop_container(&container_name, Some(10))
        .await?;
    Ok(Json(serde_json::json!({ "message": "stopped" })))
}

pub(super) async fn restart_database(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let container_name = resolve_db_container(&state, &ctx, &id).await?;
    state.docker.restart_container(&container_name).await?;
    Ok(Json(serde_json::json!({ "message": "restarted" })))
}
