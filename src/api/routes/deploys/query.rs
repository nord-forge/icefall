use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Deserialize)]
pub(super) struct LatestDeploysParams {
    app_ids: String,
}

pub(super) async fn list_deploys(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deploys = state.db.list_deploys(&id, 50).await?;
    Ok(Json(serde_json::json!({ "data": deploys })))
}

pub(super) async fn get_latest_deploys(
    State(state): State<AppState>,
    Query(params): Query<LatestDeploysParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app_ids: Vec<String> = params
        .app_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let deploys = state.db.get_latest_deploys_for_apps(&app_ids).await?;
    Ok(Json(serde_json::json!({ "data": deploys })))
}
