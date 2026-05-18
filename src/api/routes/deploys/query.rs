use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::TeamCtx;
use crate::api::AppState;

#[derive(Deserialize)]
pub(super) struct LatestDeploysParams {
    app_ids: String,
}

pub(super) async fn list_deploys(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Read-only — the app must belong to the caller's team (viewer).
    state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {id}")))?;

    let deploys = state.db.list_deploys(&id, 50).await?;
    Ok(Json(serde_json::json!({ "data": deploys })))
}

pub(super) async fn get_latest_deploys(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Query(params): Query<LatestDeploysParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let requested: Vec<String> = params
        .app_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Only query deploys for apps that belong to the caller's team.
    let team_apps: std::collections::HashSet<String> = state
        .db
        .list_apps_by_team(&ctx.team_id)
        .await?
        .into_iter()
        .map(|a| a.id)
        .collect();
    let app_ids: Vec<String> = requested
        .into_iter()
        .filter(|id| team_apps.contains(id))
        .collect();

    let deploys = state.db.get_latest_deploys_for_apps(&app_ids).await?;
    Ok(Json(serde_json::json!({ "data": deploys })))
}
