use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewProjectEnvironment;

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
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let envs = state.db.list_project_environments(&project_id).await?;
    Ok(Json(serde_json::json!({ "data": envs })))
}

#[derive(Deserialize)]
struct CreateEnvironmentRequest {
    name: String,
    color: Option<String>,
}

async fn create_environment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let slug = body.name.to_lowercase().replace(' ', "-");

    let new_env = NewProjectEnvironment {
        project_id,
        name: body.name,
        slug,
        color: body.color,
    };

    let env = state.db.create_project_environment(&new_env).await?;
    Ok(Json(serde_json::json!({ "data": env })))
}

async fn update_environment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((_project_id, env_id)): Path<(String, String)>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let existing = state.db.get_project_environment(&env_id).await?;
    if existing.is_none() {
        return Err(ApiError::NotFound(format!(
            "environment {env_id} not found"
        )));
    }

    let env = state
        .db
        .update_project_environment(&env_id, &body.name, body.color.as_deref())
        .await?;
    Ok(Json(serde_json::json!({ "data": env })))
}

async fn delete_environment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((_project_id, env_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    state.db.delete_project_environment(&env_id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn list_variables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(env_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    // Project environments use the same env_vars table through the environment_id
    // linkage. The env_id here is a project_environment ID which can be used
    // as the environment_id for env_vars storage.
    let vars = state.db.get_env_vars(&env_id).await?;
    Ok(Json(serde_json::json!({ "data": vars })))
}

#[derive(Deserialize)]
struct SetVariableRequest {
    key: String,
    value: String,
    #[allow(dead_code)]
    is_secret: Option<bool>,
}

async fn set_variable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(env_id): Path<String>,
    Json(body): Json<SetVariableRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    if body.key.trim().is_empty() {
        return Err(ApiError::BadRequest("key is required".into()));
    }

    let env_var = crate::db::models::NewEnvVar {
        environment_id: env_id,
        key: body.key,
        value: body.value,
        scope: "project_environment".to_string(),
    };

    let var = state.db.set_env_var(&env_var).await?;
    Ok(Json(serde_json::json!({ "data": var })))
}

async fn delete_variable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((_env_id, var_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    state.db.delete_env_var(&var_id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}
