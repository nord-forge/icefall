use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::now_iso8601;

pub async fn export_bundle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let app = state
        .db
        .get_app(&app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {app_id}")))?;

    let envs = state.db.list_environments(&app_id).await?;
    let env_vars = if let Some(env) = envs.first() {
        state.db.get_env_vars(&env.id).await.unwrap_or_default()
    } else {
        Vec::new()
    };

    let env_template: Vec<serde_json::Value> = env_vars
        .iter()
        .map(|v| {
            serde_json::json!({
                "key": v.key,
                "required": true,
                "scope": v.scope,
            })
        })
        .collect();

    let domains: Vec<String> = state
        .db
        .list_domains(&app_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.domain)
        .collect();

    let bundle = serde_json::json!({
        "icefall_bundle": "1.0",
        "exported_at": now_iso8601(),
        "app": {
            "name": app.name,
            "type": if app.image_ref.is_some() { "image" } else if app.compose_content.is_some() { "compose" } else { "git" },
            "repo_url": app.git_repo,
            "branch": app.git_branch,
            "framework": app.framework,
            "build_config": app.build_config,
            "deploy_mode": app.deploy_mode,
            "base_directory": app.base_directory,
            "image_ref": app.image_ref,
        },
        "env_template": env_template,
        "resources": app.resource_limits,
        "volumes": app.volumes,
        "compose": app.compose_content,
        "pre_deploy_commands": app.pre_deploy_commands,
        "post_deploy_commands": app.post_deploy_commands,
        "domain_patterns": domains,
    });

    Ok(Json(serde_json::json!({ "data": bundle })))
}

pub async fn import_bundle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(bundle): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if user.role == "viewer" {
        return Err(ApiError::Forbidden(
            "Deployer or admin role required to import bundles".into(),
        ));
    }

    let app_config = bundle
        .get("app")
        .ok_or_else(|| ApiError::BadRequest("Bundle missing 'app' field".into()))?;

    let name = app_config
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("Bundle missing app name".into()))?;

    let new_app = crate::db::models::NewApp {
        name: name.to_string(),
        git_repo: app_config
            .get("repo_url")
            .and_then(|v| v.as_str())
            .map(String::from),
        git_branch: app_config
            .get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main")
            .to_string(),
        framework: app_config
            .get("framework")
            .and_then(|v| v.as_str())
            .map(String::from),
        image_ref: app_config
            .get("image_ref")
            .and_then(|v| v.as_str())
            .map(String::from),
        compose_content: bundle
            .get("compose")
            .and_then(|v| v.as_str())
            .map(String::from),
        deploy_mode: app_config
            .get("deploy_mode")
            .and_then(|v| v.as_str())
            .map(String::from),
        server_id: None,
    };

    let app = state.db.create_app(&new_app).await?;

    Ok(Json(serde_json::json!({
        "data": app,
        "message": format!("App '{}' imported from bundle", app.name),
    })))
}
