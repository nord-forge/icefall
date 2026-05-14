use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewSharedVariable;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/shared-variables/{scope}/{scope_id}",
            get(list_variables).post(create_variable),
        )
        .route(
            "/shared-variables/{id}",
            axum::routing::delete(delete_variable),
        )
        .route("/apps/{app_id}/resolved-variables", get(resolve_variables))
}

async fn list_variables(
    State(state): State<AppState>,
    Path((scope, scope_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if scope != "project" && scope != "server" {
        return Err(ApiError::BadRequest(
            "scope must be 'project' or 'server'".into(),
        ));
    }

    let vars = state.db.list_shared_variables(&scope, &scope_id).await?;
    Ok(Json(serde_json::json!({ "data": vars })))
}

#[derive(Deserialize)]
struct CreateVariableRequest {
    key: String,
    value: String,
    #[serde(default)]
    is_sensitive: bool,
}

async fn create_variable(
    State(state): State<AppState>,
    Path((scope, scope_id)): Path<(String, String)>,
    Json(body): Json<CreateVariableRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if scope != "project" && scope != "server" {
        return Err(ApiError::BadRequest(
            "scope must be 'project' or 'server'".into(),
        ));
    }

    let var = state
        .db
        .create_shared_variable(&NewSharedVariable {
            scope,
            scope_id,
            key: body.key,
            value: body.value,
            is_sensitive: body.is_sensitive,
        })
        .await?;
    Ok(Json(serde_json::json!({ "data": var })))
}

async fn delete_variable(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.delete_shared_variable(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn resolve_variables(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let resolved = state.db.resolve_shared_variables(&app_id).await?;
    let data: Vec<serde_json::Value> = resolved
        .into_iter()
        .map(|(key, value, source)| {
            serde_json::json!({
                "key": key,
                "value": value,
                "source": source,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "data": data })))
}
