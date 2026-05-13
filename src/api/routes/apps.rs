use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::{NewApp, NewDeploy, NewEnvironment, UpdateApp, CONTROL_PLANE_SERVER_ID};
use crate::deploy::manager::DeployManager;

#[derive(Deserialize, Default)]
struct ListAppsQuery {
    tag: Option<String>,
    project_id: Option<String>,
}

#[derive(Deserialize)]
struct CreateAppRequest {
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
struct UpdateAppRequest {
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
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps", get(list_apps).post(create_app))
        .route(
            "/apps/{id}",
            get(get_app).put(update_app).delete(delete_app),
        )
        .route("/apps/{id}/start", post(start_app))
        .route("/apps/{id}/stop", post(stop_app))
        .route("/apps/{id}/restart", post(restart_app))
        .route("/apps/{id}/migrate", axum::routing::put(migrate_app))
}

async fn list_apps(
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

async fn create_app(
    State(state): State<AppState>,
    Json(body): Json<CreateAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "App name must not be empty".to_string(),
        ));
    }

    // Validate compose YAML if provided
    if let Some(ref yaml) = body.compose_content {
        if crate::deploy::compose::ComposeDeployer::parse(yaml).is_err() {
            return Err(ApiError::BadRequest(
                "Invalid Docker Compose YAML".to_string(),
            ));
        }
    }

    // Validate server_id if provided
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

    // If a port was provided (typically for image-based apps), store it in build_config
    if let Some(port) = body.port {
        let build_config = serde_json::json!({ "port": port }).to_string();
        let _ = state
            .db
            .update_app(
                &app.id,
                &UpdateApp {
                    name: None,
                    git_repo: None,
                    git_branch: None,
                    framework: None,
                    build_config: Some(build_config),
                    resource_limits: None,
                    preview_enabled: None,
                    preview_branch_pattern: None,
                    tags: None,
                    volumes: None,
                    image_ref: None,
                    compose_content: None,
                    project_id: None,
                    deploy_mode: None,
                    server_id: None,
                },
            )
            .await;
    }

    // Create a default production environment for the new app
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

async fn get_app(
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

async fn update_app(
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
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": app })))
}

async fn delete_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.delete_app(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn start_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;

    let mut started = 0u32;
    for container in &containers {
        if container.state != "running" {
            state.docker.start_container(&container.id).await?;
            started += 1;
        }
    }

    Ok(Json(
        serde_json::json!({ "message": "started", "containers": started }),
    ))
}

async fn stop_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;

    let mut stopped = 0u32;
    for container in &containers {
        if container.state == "running" {
            state.docker.stop_container(&container.id, Some(10)).await?;
            stopped += 1;
        }
    }

    Ok(Json(
        serde_json::json!({ "message": "stopped", "containers": stopped }),
    ))
}

async fn restart_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;

    let mut restarted = 0u32;
    for container in &containers {
        if container.state == "running" {
            state.docker.restart_container(&container.id).await?;
            restarted += 1;
        }
    }

    Ok(Json(
        serde_json::json!({ "message": "restarted", "containers": restarted }),
    ))
}

#[derive(Deserialize)]
struct MigrateAppRequest {
    target_server_id: String,
    #[serde(default)]
    acknowledge_volume_loss: bool,
}

async fn migrate_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<MigrateAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let current_server_id = app.server_id.as_deref().unwrap_or(CONTROL_PLANE_SERVER_ID);

    if body.target_server_id == current_server_id {
        return Err(ApiError::BadRequest(
            "Target server is the same as the current server".into(),
        ));
    }

    let target_server = state
        .db
        .get_server(&body.target_server_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Server '{}' not found", body.target_server_id))
        })?;

    if target_server.status != "online" {
        return Err(ApiError::BadRequest(format!(
            "Target server '{}' is not connected (status: {})",
            target_server.name, target_server.status
        )));
    }

    if app.volumes.as_ref().is_some_and(|v| !v.is_empty()) && !body.acknowledge_volume_loss {
        return Err(ApiError::BadRequest(
            "This app has volumes. Volumes are NOT migrated between servers. \
             Set acknowledge_volume_loss: true to proceed."
                .into(),
        ));
    }

    let envs = state.db.list_environments(&app.id).await?;
    let env = envs
        .first()
        .ok_or_else(|| ApiError::BadRequest("App has no environments".into()))?
        .clone();

    let deploy = state
        .db
        .create_deploy(&NewDeploy {
            app_id: app.id.clone(),
            environment_id: env.id.clone(),
            git_sha: None,
            server_id: Some(body.target_server_id.clone()),
        })
        .await?;

    let deploy_id = deploy.id.clone();
    let target_server_id = body.target_server_id.clone();
    let source_server_id = current_server_id.to_string();

    let mut warnings = Vec::new();
    if app.volumes.as_ref().is_some_and(|v| !v.is_empty()) {
        warnings.push(
            "Volumes are not migrated. Data on the source server's volumes will remain there."
                .to_string(),
        );
    }

    tokio::spawn(async move {
        let manager = DeployManager::new(
            state.docker.clone(),
            state.caddy.clone(),
            state.db.clone(),
            state.config.clone(),
            state.event_bus.clone(),
            Some(state.agent_registry.clone()),
        );

        // Temporarily set app's server_id to target for the deploy
        let _ = state
            .db
            .update_app(
                &app.id,
                &UpdateApp {
                    server_id: Some(Some(target_server_id.clone())),
                    ..Default::default()
                },
            )
            .await;

        // Re-fetch app with updated server_id
        let target_app = match state.db.get_app(&app.id).await {
            Ok(Some(a)) => a,
            _ => return,
        };

        let git_repo = match target_app.git_repo.as_deref() {
            Some(r) => r.to_string(),
            None => {
                tracing::error!("Migration failed: no git_repo for app {}", app.id);
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some("No git_repo"))
                    .await;
                // Revert server_id
                let _ = state
                    .db
                    .update_app(
                        &app.id,
                        &UpdateApp {
                            server_id: Some(Some(source_server_id)),
                            ..Default::default()
                        },
                    )
                    .await;
                return;
            }
        };

        // Build on target server
        let target = manager.resolve_target(&target_app);
        let image_ref = match target {
            crate::deploy::DeployTarget::Remote { ref server_id } => {
                let executor = match manager.make_remote_executor(server_id).await {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("Migration build failed: {e}");
                        let _ = state
                            .db
                            .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                            .await;
                        let _ = state
                            .db
                            .update_app(
                                &app.id,
                                &UpdateApp {
                                    server_id: Some(Some(source_server_id)),
                                    ..Default::default()
                                },
                            )
                            .await;
                        return;
                    }
                };

                let timeout = std::time::Duration::from_secs(state.config.build_timeout_secs);
                match executor
                    .run_build(
                        &git_repo,
                        &target_app.git_branch,
                        &deploy_id,
                        &target_app.name,
                        &[],
                        None,
                        timeout,
                    )
                    .await
                {
                    Ok(tag) => tag,
                    Err(e) => {
                        tracing::error!("Migration build failed: {e}");
                        let _ = state
                            .db
                            .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                            .await;
                        let _ = state
                            .db
                            .update_app(
                                &app.id,
                                &UpdateApp {
                                    server_id: Some(Some(source_server_id)),
                                    ..Default::default()
                                },
                            )
                            .await;
                        return;
                    }
                }
            }
            crate::deploy::DeployTarget::Local => {
                let orchestrator = crate::build::orchestrator::BuildOrchestrator::new(
                    state.docker.clone(),
                    state.db.clone(),
                    state.config.clone(),
                );
                match orchestrator.build(&deploy_id, &target_app, None).await {
                    Ok(result) => result.image_ref,
                    Err(e) => {
                        tracing::error!("Migration build failed: {e}");
                        let _ = state
                            .db
                            .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                            .await;
                        let _ = state
                            .db
                            .update_app(
                                &app.id,
                                &UpdateApp {
                                    server_id: Some(Some(source_server_id)),
                                    ..Default::default()
                                },
                            )
                            .await;
                        return;
                    }
                }
            }
        };

        // Deploy on target
        let updated_deploy = match state.db.get_deploy(&deploy_id).await {
            Ok(Some(d)) => d,
            _ => return,
        };

        let env_clone = env.clone();
        if let Err(e) = manager
            .deploy(&updated_deploy, &target_app, &env_clone, &image_ref)
            .await
        {
            tracing::error!("Migration deploy failed: {e}");
            let _ = state
                .db
                .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                .await;
            // Revert server_id on failure
            let _ = state
                .db
                .update_app(
                    &app.id,
                    &UpdateApp {
                        server_id: Some(Some(source_server_id)),
                        ..Default::default()
                    },
                )
                .await;
            return;
        }

        // Success — stop containers on source server
        if source_server_id != CONTROL_PLANE_SERVER_ID {
            if let Ok(source_exec) = manager.make_remote_executor(&source_server_id).await {
                let _ = source_exec
                    .remove_caddy_route(&format!(
                        "{}.{}",
                        app.name,
                        state.config.base_domain.as_deref().unwrap_or("")
                    ))
                    .await;
            }
        }

        tracing::info!(
            "Migration complete: app {} moved from {} to {}",
            app.name,
            source_server_id,
            target_server_id
        );
    });

    Ok(Json(serde_json::json!({
        "data": deploy,
        "warnings": warnings,
        "message": "Migration started",
    })))
}
