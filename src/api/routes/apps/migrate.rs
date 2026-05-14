use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::{NewDeploy, UpdateApp, CONTROL_PLANE_SERVER_ID};
use crate::deploy::manager::DeployManager;

#[derive(Deserialize)]
pub(super) struct MigrateAppRequest {
    target_server_id: String,
    #[serde(default)]
    acknowledge_volume_loss: bool,
}

pub(super) async fn migrate_app(
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
            no_cache: false,
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

        let Ok(Some(target_app)) = state.db.get_app(&app.id).await else {
            return;
        };

        let git_repo = match target_app.git_repo.as_deref() {
            Some(r) => r.to_string(),
            None => {
                tracing::error!("Migration failed: no git_repo for app {}", app.id);
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some("No git_repo"))
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
                match orchestrator
                    .build(&deploy_id, &target_app, None, false)
                    .await
                {
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

        let Ok(Some(updated_deploy)) = state.db.get_deploy(&deploy_id).await else {
            return;
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
