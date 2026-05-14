use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct CloneAppRequest {
    new_name: String,
    target_project_id: Option<String>,
    target_server_id: Option<String>,
}

pub async fn clone_app(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
    Json(body): Json<CloneAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _source = state
        .db
        .get_app(&app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {app_id}")))?;

    let cloned = state
        .db
        .clone_app(
            &app_id,
            &body.new_name,
            body.target_project_id.as_deref(),
            body.target_server_id.as_deref(),
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": cloned })))
}

#[derive(Deserialize)]
pub struct MoveAppRequest {
    target_project_id: Option<String>,
}

pub async fn move_app(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
    Json(body): Json<MoveAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .update_app(
            &app_id,
            &crate::db::models::UpdateApp {
                project_id: Some(body.target_project_id),
                ..Default::default()
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": app })))
}
