use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::{NewApp, NewEnvironment, UpdateApp, CONTROL_PLANE_SERVER_ID};

#[derive(Deserialize, Default)]
pub(super) struct ListAppsQuery {
    tag: Option<String>,
    project_id: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct CreateAppRequest {
    name: String,
    git_repo: Option<String>,
    git_branch: Option<String>,
    framework: Option<String>,
    image_ref: Option<String>,
    compose_content: Option<String>,
    port: Option<u16>,
    deploy_mode: Option<String>,
    server_id: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct UpdateAppRequest {
    name: Option<String>,
    git_repo: Option<String>,
    git_branch: Option<String>,
    framework: Option<String>,
    build_config: Option<String>,
    resource_limits: Option<String>,
    preview_enabled: Option<bool>,
    preview_branch_pattern: Option<Option<String>>,
    tags: Option<String>,
    volumes: Option<String>,
    image_ref: Option<Option<String>>,
    compose_content: Option<Option<String>>,
    project_id: Option<Option<String>>,
    deploy_mode: Option<String>,
    base_directory: Option<Option<String>>,
    disable_build_cache: Option<bool>,
    git_submodules_enabled: Option<bool>,
    git_lfs_enabled: Option<bool>,
    git_shallow_clone: Option<bool>,
    basic_auth_enabled: Option<bool>,
    basic_auth_username: Option<Option<String>>,
    basic_auth_password: Option<String>,
    pre_deploy_commands: Option<Option<String>>,
    post_deploy_commands: Option<Option<String>>,
    ssh_key_id: Option<Option<String>>,
}

pub(super) async fn list_apps(
    State(state): State<AppState>,
    Query(query): Query<ListAppsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut apps = if let Some(pid) = &query.project_id {
        state.db.list_apps_by_project(pid).await?
    } else {
        state.db.list_apps().await?
    };

    if let Some(tag) = &query.tag {
        let tag = tag.trim().to_lowercase();
        if !tag.is_empty() {
            apps.retain(|app| {
                app.tags
                    .as_deref()
                    .unwrap_or("")
                    .split(',')
                    .any(|t| t.trim() == tag)
            });
        }
    }

    Ok(Json(serde_json::json!({ "data": apps })))
}

pub(super) async fn create_app(
    State(state): State<AppState>,
    Json(body): Json<CreateAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "App name must not be empty".to_string(),
        ));
    }

    if let Some(ref yaml) = body.compose_content {
        if crate::deploy::compose::ComposeDeployer::parse(yaml).is_err() {
            return Err(ApiError::BadRequest(
                "Invalid Docker Compose YAML".to_string(),
            ));
        }
    }

    let resolved_server_id = if let Some(ref sid) = body.server_id {
        if sid != CONTROL_PLANE_SERVER_ID {
            let server = state
                .db
                .get_server(sid)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("Server {sid} not found")))?;
            if server.status == "offline" || server.status == "enrolling" {
                return Err(ApiError::BadRequest(format!(
                    "Server '{}' is not connected (status: {})",
                    server.name, server.status
                )));
            }
            if server.role == "draining" {
                return Err(ApiError::BadRequest(format!(
                    "Server '{}' is draining and cannot accept new apps",
                    server.name
                )));
            }
        }
        Some(sid.clone())
    } else {
        None
    };

    let app = state
        .db
        .create_app(&NewApp {
            name: body.name.clone(),
            git_repo: body.git_repo,
            git_branch: body.git_branch.unwrap_or_else(|| "main".into()),
            framework: body.framework,
            image_ref: body.image_ref,
            compose_content: body.compose_content,
            deploy_mode: body.deploy_mode,
            server_id: resolved_server_id,
        })
        .await?;

    if let Some(port) = body.port {
        let build_config = serde_json::json!({ "port": port }).to_string();
        let _ = state
            .db
            .update_app(
                &app.id,
                &UpdateApp {
                    build_config: Some(build_config),
                    ..Default::default()
                },
            )
            .await;
    }

    let _ = state
        .db
        .create_environment(&NewEnvironment {
            app_id: app.id.clone(),
            name: "production".into(),
            env_type: "production".into(),
            branch: None,
        })
        .await?;

    Ok(Json(serde_json::json!({ "data": app })))
}

pub(super) async fn get_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    Ok(Json(serde_json::json!({ "data": app })))
}

pub(super) async fn update_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .update_app(
            &id,
            &UpdateApp {
                name: body.name,
                git_repo: body.git_repo,
                git_branch: body.git_branch,
                framework: body.framework,
                build_config: body.build_config,
                resource_limits: body.resource_limits,
                preview_enabled: body.preview_enabled,
                preview_branch_pattern: body.preview_branch_pattern,
                tags: body.tags.map(|t| {
                    t.split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(",")
                }),
                volumes: body.volumes,
                image_ref: body.image_ref,
                compose_content: body.compose_content,
                project_id: body.project_id,
                deploy_mode: body.deploy_mode,
                server_id: None,
                base_directory: body.base_directory,
                disable_build_cache: body.disable_build_cache,
                git_submodules_enabled: body.git_submodules_enabled,
                git_lfs_enabled: body.git_lfs_enabled,
                git_shallow_clone: body.git_shallow_clone,
                basic_auth_enabled: body.basic_auth_enabled,
                basic_auth_username: body.basic_auth_username,
                basic_auth_password_hash: body
                    .basic_auth_password
                    .map(|pw| Some(bcrypt::hash(pw, bcrypt::DEFAULT_COST).unwrap_or_default())),
                pre_deploy_commands: body.pre_deploy_commands,
                post_deploy_commands: body.post_deploy_commands,
                ssh_key_id: body.ssh_key_id,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": app })))
}

pub(super) async fn delete_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.delete_app(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}
