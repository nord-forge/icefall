use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

#[derive(Deserialize)]
struct AuditQuery {
    limit: Option<u32>,
    offset: Option<u32>,
    action: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/audit-log", get(list_all_audit_logs))
        .route("/servers/{id}/audit-log", get(list_server_audit_logs))
}

async fn list_all_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let entries = state
        .db
        .list_audit_logs(None, query.action.as_deref(), limit, offset)
        .await?;

    Ok(Json(serde_json::json!({ "data": entries })))
}

async fn list_server_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let entries = state
        .db
        .list_audit_logs(Some(&server_id), query.action.as_deref(), limit, offset)
        .await?;

    Ok(Json(serde_json::json!({ "data": entries })))
}
