use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{id}/environments",
            get(list_environments).post(create_environment),
        )
        .route(
            "/projects/{project_id}/environments/{env_id}",
            put(update_environment).delete(delete_environment),
        )
        .route(
            "/environments/{id}/variables",
            get(list_variables).post(set_variable),
        )
        .route(
            "/environments/{env_id}/variables/{var_id}",
            delete(delete_variable),
        )
}

async fn list_environments(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_project_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&_state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "data": [] })))
}

#[derive(Deserialize)]
struct CreateEnvironmentRequest {
    name: String,
    color: Option<String>,
}

async fn create_environment(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_project_id): Path<String>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&_state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": uuid::Uuid::new_v4().to_string(),
            "project_id": _project_id,
            "name": body.name,
            "color": body.color,
            "created_at": chrono::Utc::now().to_rfc3339(),
        }
    })))
}

async fn update_environment(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path((_project_id, env_id)): Path<(String, String)>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&_state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": env_id,
            "project_id": _project_id,
            "name": body.name,
            "color": body.color,
        }
    })))
}

async fn delete_environment(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path((_project_id, _env_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&_state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn list_variables(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_env_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&_state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "data": [] })))
}

#[derive(Deserialize)]
struct SetVariableRequest {
    key: String,
    value: String,
    is_secret: Option<bool>,
}

async fn set_variable(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_env_id): Path<String>,
    Json(body): Json<SetVariableRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&_state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": uuid::Uuid::new_v4().to_string(),
            "key": body.key,
            "value": body.value,
            "is_secret": body.is_secret.unwrap_or(false),
        }
    })))
}

async fn delete_variable(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path((_env_id, _var_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&_state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}
