use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewSharedVariable;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/shared-variables/{scope}/{scope_id}",
            get(list_variables).post(create_variable),
        )
        .route("/shared-variables/{id}", delete(delete_variable))
}

#[derive(Debug, Deserialize)]
struct CreateVariableRequest {
    key: String,
    value: String,
    #[serde(default)]
    is_sensitive: bool,
}

async fn list_variables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((scope, scope_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    validate_scope(&scope)?;
    validate_scope_id_exists(&state, &scope, &scope_id).await?;

    let vars = state.db.list_shared_variables(&scope, &scope_id).await?;

    let data: Vec<serde_json::Value> = vars
        .into_iter()
        .map(|v| {
            serde_json::json!({
                "id": v.id,
                "scope": v.scope,
                "scope_id": v.scope_id,
                "key": v.key,
                "value": if v.is_sensitive { "••••••••".to_string() } else { v.value },
                "is_sensitive": v.is_sensitive,
                "created_at": v.created_at,
                "updated_at": v.updated_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": data })))
}

async fn create_variable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((scope, scope_id)): Path<(String, String)>,
    Json(body): Json<CreateVariableRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    validate_scope(&scope)?;
    validate_key(&body.key)?;
    validate_scope_id_exists(&state, &scope, &scope_id).await?;

    let var = state
        .db
        .set_shared_variable(&NewSharedVariable {
            scope,
            scope_id,
            key: body.key,
            value: body.value,
            is_sensitive: body.is_sensitive,
        })
        .await?;

    Ok(Json(serde_json::json!({ "data": {
        "id": var.id,
        "key": var.key,
        "scope": var.scope,
        "scope_id": var.scope_id,
        "is_sensitive": var.is_sensitive,
    }})))
}

async fn delete_variable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    state.db.delete_shared_variable(&id).await?;

    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

fn validate_scope(scope: &str) -> Result<(), ApiError> {
    match scope {
        "project" | "server" => Ok(()),
        _ => Err(ApiError::BadRequest(format!(
            "invalid scope '{scope}': must be 'project' or 'server'"
        ))),
    }
}

fn validate_key(key: &str) -> Result<(), ApiError> {
    if key.is_empty() {
        return Err(ApiError::BadRequest("key cannot be empty".into()));
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(ApiError::BadRequest(
            "key must contain only uppercase letters, digits, and underscores".into(),
        ));
    }
    Ok(())
}

async fn validate_scope_id_exists(
    state: &AppState,
    scope: &str,
    scope_id: &str,
) -> Result<(), ApiError> {
    match scope {
        "project" => {
            state
                .db
                .get_project(scope_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("project '{scope_id}' not found")))?;
        }
        "server" => {
            state
                .db
                .get_server(scope_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("server '{scope_id}' not found")))?;
        }
        _ => {}
    }
    Ok(())
}
