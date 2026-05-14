use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::deploy::drift::compute_config_hash;

pub(super) async fn check_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {id}")))?;

    let envs = state.db.list_environments(&id).await?;
    let env_vars: Vec<(String, String)> = if let Some(env) = envs.first() {
        state
            .db
            .get_env_vars(&env.id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|v| (v.key, v.value))
            .collect()
    } else {
        Vec::new()
    };

    let domains: Vec<String> = state
        .db
        .list_domains(&id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.domain)
        .collect();

    let current_hash = compute_config_hash(&app, &env_vars, &domains);

    let deploys = state.db.list_deploys(&id, 10).await?;
    let last_running = deploys.iter().find(|d| d.status == "running");

    let (drifted, deployed_hash) = match last_running {
        Some(deploy) => match &deploy.config_hash {
            Some(hash) => (hash != &current_hash, Some(hash.clone())),
            None => (false, None),
        },
        None => (false, None),
    };

    Ok(Json(serde_json::json!({
        "data": {
            "drifted": drifted,
            "current_hash": current_hash,
            "deployed_hash": deployed_hash,
        }
    })))
}
