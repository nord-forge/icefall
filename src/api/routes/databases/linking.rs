use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewEnvVar;

use super::config::db_configs;

pub(super) async fn link_to_app(
    State(state): State<AppState>,
    Path((id, app_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dbs = state.db.list_managed_dbs().await?;
    let db = dbs
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;

    let configs = db_configs();
    let type_config = configs
        .get(db.db_type.as_str())
        .ok_or_else(|| ApiError::internal(std::io::Error::other("unknown db type")))?;

    let creds: serde_json::Value = serde_json::from_str(&db.credentials).unwrap_or_default();
    let conn_str = creds
        .get("connection_string")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let envs = state.db.list_environments(&app_id).await?;
    if let Some(env) = envs.first() {
        state
            .db
            .set_env_var(&NewEnvVar {
                environment_id: env.id.clone(),
                key: type_config.env_var_name.to_string(),
                value: conn_str.to_string(),
                scope: "shared".to_string(),
            })
            .await?;
    }

    Ok(Json(
        serde_json::json!({ "message": "linked", "env_var": type_config.env_var_name }),
    ))
}

pub(super) async fn unlink_from_app(
    State(state): State<AppState>,
    Path((_id, app_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let envs = state.db.list_environments(&app_id).await?;
    if let Some(env) = envs.first() {
        let vars = state.db.get_env_vars(&env.id).await?;
        for var in vars {
            if var.key == "DATABASE_URL" || var.key == "REDIS_URL" || var.key == "MONGODB_URL" {
                state.db.delete_env_var(&var.id).await?;
            }
        }
    }

    Ok(Json(serde_json::json!({ "message": "unlinked" })))
}
