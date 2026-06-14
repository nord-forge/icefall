use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct HistoryQuery {
    limit: Option<i64>,
}

pub async fn list_app_config_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let limit = query.limit.unwrap_or(50);
    let history = state.db.list_config_history("app", &app_id, limit).await?;
    Ok(Json(serde_json::json!({ "data": history })))
}

pub async fn list_deploy_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(deploy_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let events = state.db.list_deploy_events(&deploy_id).await?;
    Ok(Json(serde_json::json!({ "data": events })))
}

pub async fn approve_deploy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(deploy_id): Path<String>,
    Json(body): Json<ApprovalRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    if user.role != "admin" {
        return Err(ApiError::Forbidden(
            "Admin role required to approve deploys".into(),
        ));
    }

    let deploy = state
        .db
        .get_deploy(&deploy_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;

    if deploy.status != "pending_approval" {
        return Err(ApiError::BadRequest(format!(
            "Deploy status is '{}', not 'pending_approval'",
            deploy.status
        )));
    }

    let approval = state
        .db
        .create_deploy_approval(&deploy_id, &body.action, &user.id, body.comment.as_deref())
        .await?;

    if body.action == "approved" {
        state
            .db
            .update_deploy_status(&deploy_id, "pending", Some("Approved, queued for deploy"))
            .await?;
    } else {
        state
            .db
            .update_deploy_status(&deploy_id, "cancelled", Some("Rejected by reviewer"))
            .await?;
    }

    Ok(Json(serde_json::json!({ "data": approval })))
}

#[derive(Deserialize)]
pub struct ApprovalRequest {
    action: String,
    comment: Option<String>,
}
