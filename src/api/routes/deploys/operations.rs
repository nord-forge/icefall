use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::build::orchestrator::BuildOrchestrator;
use crate::build::BuildConfig;
use crate::db::models::{App, Environment, NewDeploy};
use crate::deploy::compose::ComposeDeployer;
use crate::deploy::manager::DeployManager;
use crate::deploy::native::NativeDeployer;

#[derive(Deserialize, Default)]
pub(super) struct CreateDeployRequest {
    pub no_cache: Option<bool>,
    /// Optional ISO 8601 (UTC) trigger time. When present and in the future the
    /// deploy is queued in the `scheduled` state instead of running now (IF-179).
    pub scheduled_at: Option<String>,
}

/// Validate a caller-supplied schedule time (RFC 3339, in the future). Returns the
/// normalized UTC string matching `now_iso8601()` so lexical comparison stays correct.
fn validate_scheduled_at(raw: &str) -> Result<String, ApiError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(raw).map_err(|_| {
        ApiError::BadRequest("scheduled_at must be an ISO 8601 / RFC 3339 timestamp".into())
    })?;
    let when = parsed.with_timezone(&chrono::Utc);
    if when <= chrono::Utc::now() {
        return Err(ApiError::BadRequest(
            "scheduled_at must be in the future".into(),
        ));
    }
    Ok(when.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

pub(super) async fn create_deploy(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    body: Option<Json<CreateDeployRequest>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The app must belong to the caller's team, member role to deploy.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {id}")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    let is_compose_deploy = app.compose_content.is_some();
    let is_image_deploy = app.image_ref.is_some();

    if !is_compose_deploy && !is_image_deploy {
        app.git_repo
            .as_ref()
            .ok_or_else(|| ApiError::BadRequest("app has no git_repo configured".into()))?;
    }

    let request_no_cache = body.as_ref().and_then(|b| b.no_cache).unwrap_or(false);
    let no_cache = request_no_cache || app.disable_build_cache;

    let scheduled_at = body
        .as_ref()
        .and_then(|b| b.scheduled_at.as_deref())
        .map(validate_scheduled_at)
        .transpose()?;

    let envs = state.db.list_environments(&id).await?;
    let env = envs
        .first()
        .ok_or_else(|| ApiError::BadRequest("app has no environments".into()))?;

    let deploy = state
        .db
        .create_deploy(&NewDeploy {
            app_id: id.clone(),
            environment_id: env.id.clone(),
            git_sha: None,
            server_id: None,
            tag: None,
            no_cache,
            scheduled_at: scheduled_at.clone(),
        })
        .await?;

    // Scheduled: leave it queued and let the deploy scheduler fire it later.
    if let Some(when) = scheduled_at {
        state.event_bus.emit(
            crate::events::EventType::DeployStatus,
            Some(&id),
            Some(&deploy.id),
            serde_json::json!({ "status": "scheduled", "scheduled_at": when }),
        );
        crate::api::routes::notifications::emit_event(
            &state.db,
            &state.config.caddy_admin_url,
            "deploy.scheduled",
            Some(&id),
            &format!("Deploy scheduled for app '{}' at {when}", app.name),
            serde_json::json!({ "app": app.name, "deploy_id": deploy.id, "scheduled_at": when }),
        )
        .await;
        return Ok(Json(serde_json::json!({ "data": deploy })));
    }

    trigger_deploy(state, app, env.clone(), deploy.id.clone(), no_cache).await;

    Ok(Json(serde_json::json!({ "data": deploy })))
}

/// Spawn the deploy pipeline for an already-created deploy record (IF-179). Picks
/// the compose/image/native/auto path and runs it in a background task.
pub(crate) async fn trigger_deploy(
    state: AppState,
    app: App,
    env: Environment,
    deploy_id: String,
    no_cache: bool,
) {
    // IF-174: report GitHub commit status across all deploy paths. The watcher
    // tracks the deploy's terminal DB state, so it needn't hook each branch.
    if let Ok(Some(d)) = state.db.get_deploy(&deploy_id).await {
        crate::github::status::watch_deploy(&state, &app, &deploy_id, d.git_sha.as_deref());
    }

    let is_compose_deploy = app.compose_content.is_some();
    let is_image_deploy = app.image_ref.is_some();

    let app_clone = app.clone();
    let env_clone = env.clone();

    // Raw Compose mode (IF-173): hand the file to the `docker compose` CLI
    // instead of Icefall's managed per-service container pipeline.
    let is_raw_compose = is_compose_deploy && app.deploy_mode == "raw-compose";

    if is_raw_compose {
        let compose_yaml = app
            .compose_content
            .clone()
            .expect("guarded by is_compose_deploy");

        tokio::spawn(async move {
            let deployer = crate::deploy::raw_compose::RawComposeDeployer::new(
                state.db.clone(),
                state.event_bus.clone(),
                state.config.clone(),
            );

            let env_map: std::collections::HashMap<String, String> = state
                .db
                .get_env_vars(&env_clone.id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|v| (v.key, v.value))
                .collect();

            if let Err(e) = deployer
                .deploy(&app_clone, &deploy_id, &compose_yaml, &env_map)
                .await
            {
                tracing::error!("Raw compose deploy failed for {deploy_id}: {e}");
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                    .await;
            }
        });
    } else if is_compose_deploy {
        let compose_yaml = app
            .compose_content
            .clone()
            .expect("guarded by is_compose_deploy");

        tokio::spawn(async move {
            let deployer = ComposeDeployer::new(
                state.docker.clone(),
                state.db.clone(),
                state.event_bus.clone(),
            );

            let env_vars_list = state
                .db
                .get_env_vars(&env_clone.id)
                .await
                .unwrap_or_default();
            let env_map: std::collections::HashMap<String, String> = env_vars_list
                .into_iter()
                .map(|v| (v.key, v.value))
                .collect();

            if let Err(e) = deployer
                .deploy(&app_clone, &deploy_id, &compose_yaml, &env_map)
                .await
            {
                tracing::error!("Compose deploy failed for {deploy_id}: {e}");
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                    .await;
            }
        });
    } else if is_image_deploy {
        let image_ref = app.image_ref.clone().expect("guarded by is_image_deploy");

        tokio::spawn(async move {
            let manager = DeployManager::new(
                state.docker.clone(),
                state.caddy.clone(),
                state.db.clone(),
                state.config.clone(),
                state.event_bus.clone(),
                Some(state.agent_registry.clone()),
            );

            let Ok(Some(updated_deploy)) = state.db.get_deploy(&deploy_id).await else {
                return;
            };

            if let Err(e) = manager
                .deploy(&updated_deploy, &app_clone, &env_clone, &image_ref)
                .await
            {
                tracing::error!("Image deploy failed for {deploy_id}: {e}");
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                    .await;
            }
        });
    } else if app.deploy_mode == "native" {
        let build_config: Option<BuildConfig> = app
            .build_config
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        let lock = state.build_locks.acquire(&app.id).await;

        tokio::spawn(async move {
            let _guard = lock.lock().await;

            let deployer = NativeDeployer::new(
                state.caddy.clone(),
                state.db.clone(),
                state.config.clone(),
                state.event_bus.clone(),
            );

            let Ok(Some(updated_deploy)) = state.db.get_deploy(&deploy_id).await else {
                return;
            };

            if let Err(e) = deployer
                .deploy(&updated_deploy, &app_clone, &env_clone, build_config)
                .await
            {
                tracing::error!("Native deploy failed for {deploy_id}: {e}");
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                    .await;
            }
        });
    } else {
        let build_config: Option<BuildConfig> = app
            .build_config
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        let lock = state.build_locks.acquire(&app.id).await;
        let is_auto_mode = app.deploy_mode == "auto";

        tokio::spawn(async move {
            let _guard = lock.lock().await;

            if is_auto_mode {
                let work_dir = state.config.data_dir.join("builds").join(&deploy_id);
                let Some(git_repo) = app_clone.git_repo.as_deref() else {
                    tracing::error!("Auto-mode deploy failed: no git_repo for {deploy_id}");
                    let _ = state
                        .db
                        .update_deploy_status(
                            &deploy_id,
                            "failed",
                            Some("No git repository configured"),
                        )
                        .await;
                    return;
                };

                let clone_opts = crate::build::git::GitCloneOptions {
                    repo_url: git_repo.to_string(),
                    branch: Some(app_clone.git_branch.clone()),
                    sha: None,
                    ssh_key_path: None,
                    token: None,
                    submodules: app_clone.git_submodules_enabled,
                    lfs: app_clone.git_lfs_enabled,
                    shallow: app_clone.git_shallow_clone,
                };

                match crate::build::git::clone_repo(&clone_opts, &work_dir).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Auto-mode clone failed for {deploy_id}: {e}");
                        let _ = state
                            .db
                            .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                            .await;
                        return;
                    }
                }

                let detection = crate::build::detect::detect(&work_dir, build_config.as_ref());
                // Use native only when the framework is eligible AND the host
                // has the node toolchain; otherwise fall back to the container
                // build (which ships node in the image).
                let use_native = detection
                    .as_ref()
                    .is_ok_and(crate::deploy::native::can_use_native);

                let _ = tokio::fs::remove_dir_all(&work_dir).await;

                if use_native {
                    let deployer = NativeDeployer::new(
                        state.caddy.clone(),
                        state.db.clone(),
                        state.config.clone(),
                        state.event_bus.clone(),
                    );

                    let Ok(Some(updated_deploy)) = state.db.get_deploy(&deploy_id).await else {
                        tracing::error!("Failed to re-fetch deploy {deploy_id} for native deploy");
                        let _ = state
                            .db
                            .update_deploy_status(
                                &deploy_id,
                                "failed",
                                Some("Internal error: deploy record not found"),
                            )
                            .await;
                        return;
                    };

                    if let Err(e) = deployer
                        .deploy(
                            &updated_deploy,
                            &app_clone,
                            &env_clone,
                            build_config.clone(),
                        )
                        .await
                    {
                        tracing::error!("Native deploy failed for {deploy_id}: {e}");
                        let _ = state
                            .db
                            .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                            .await;
                    }
                    return;
                }
            }

            let manager = DeployManager::new(
                state.docker.clone(),
                state.caddy.clone(),
                state.db.clone(),
                state.config.clone(),
                state.event_bus.clone(),
                Some(state.agent_registry.clone()),
            );

            let target = manager.resolve_target(&app_clone);

            let image_ref = match target {
                crate::deploy::DeployTarget::Remote { ref server_id } => {
                    let executor = match manager.make_remote_executor(server_id).await {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::error!("Cannot reach server for deploy {deploy_id}: {e}");
                            let _ = state
                                .db
                                .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                                .await;
                            return;
                        }
                    };

                    let Some(git_repo) = app_clone.git_repo.as_deref() else {
                        tracing::error!("Remote deploy failed: no git_repo for {deploy_id}");
                        let _ = state
                            .db
                            .update_deploy_status(
                                &deploy_id,
                                "failed",
                                Some("No git repository configured"),
                            )
                            .await;
                        return;
                    };

                    let timeout = std::time::Duration::from_secs(
                        build_config
                            .as_ref()
                            .and_then(|c| c.build_timeout_secs)
                            .unwrap_or(state.config.build_timeout_secs),
                    );

                    let config_json = build_config
                        .as_ref()
                        .and_then(|c| serde_json::to_value(c).ok());

                    match executor
                        .run_build(
                            git_repo,
                            &app_clone.git_branch,
                            &deploy_id,
                            &app_clone.name,
                            &[],
                            config_json.as_ref(),
                            timeout,
                        )
                        .await
                    {
                        Ok(tag) => tag,
                        Err(e) => {
                            tracing::error!("Remote build failed for {deploy_id}: {e}");
                            let _ = state
                                .db
                                .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                                .await;
                            return;
                        }
                    }
                }
                crate::deploy::DeployTarget::Local => {
                    let orchestrator = BuildOrchestrator::new(
                        state.docker.clone(),
                        state.db.clone(),
                        state.config.clone(),
                    );

                    match orchestrator
                        .build(&deploy_id, &app_clone, build_config, no_cache)
                        .await
                    {
                        Ok(result) => result.image_ref,
                        Err(e) => {
                            tracing::error!("Build failed for deploy {deploy_id}: {e}");
                            let _ = state
                                .db
                                .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                                .await;
                            return;
                        }
                    }
                }
            };

            let Ok(Some(updated_deploy)) = state.db.get_deploy(&deploy_id).await else {
                tracing::error!("Failed to re-fetch deploy {deploy_id} after build");
                let _ = state
                    .db
                    .update_deploy_status(
                        &deploy_id,
                        "failed",
                        Some("Internal error: deploy record not found after build"),
                    )
                    .await;
                return;
            };

            if let Err(e) = manager
                .deploy(&updated_deploy, &app_clone, &env_clone, &image_ref)
                .await
            {
                tracing::error!("Deploy failed for {deploy_id}: {e}");
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                    .await;
            }
        });
    }
}

pub(super) async fn rollback_deploy(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path((app_id, deploy_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The app must belong to the caller's team, member role to roll back.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {app_id}")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    let target_deploy = state
        .db
        .get_deploy(&deploy_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;

    // IDOR: the deploy must belong to the app named in the path.
    if target_deploy.app_id != app_id {
        return Err(ApiError::NotFound(format!("deploy {deploy_id}")));
    }

    let image_ref = target_deploy
        .image_ref
        .as_ref()
        .ok_or_else(|| {
            ApiError::BadRequest("Target deploy has no image reference — cannot rollback".into())
        })?
        .clone();

    let envs = state.db.list_environments(&app_id).await?;
    let env = envs
        .first()
        .ok_or_else(|| ApiError::BadRequest("app has no environments".into()))?;

    let rollback_deploy = state
        .db
        .create_deploy(&NewDeploy {
            app_id: app_id.clone(),
            environment_id: env.id.clone(),
            git_sha: target_deploy.git_sha.clone(),
            server_id: None,
            tag: None,
            no_cache: false,
            scheduled_at: None,
        })
        .await?;

    let env_snapshot: Option<Vec<String>> = target_deploy
        .env_snapshot
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let rollback_id = rollback_deploy.id.clone();
    let env_clone = env.clone();

    // IF-174: GitHub commit status for the rollback deploy (best-effort).
    crate::github::status::watch_deploy(
        &state,
        &app,
        &rollback_id,
        rollback_deploy.git_sha.as_deref(),
    );

    tokio::spawn(async move {
        let manager = DeployManager::new(
            state.docker.clone(),
            state.caddy.clone(),
            state.db.clone(),
            state.config.clone(),
            state.event_bus.clone(),
            Some(state.agent_registry.clone()),
        );

        let Ok(Some(deploy)) = state.db.get_deploy(&rollback_id).await else {
            return;
        };

        if let Err(e) = manager
            .deploy_with_env(&deploy, &app, &env_clone, &image_ref, env_snapshot)
            .await
        {
            tracing::error!("Rollback deploy failed for {rollback_id}: {e}");
            let _ = state
                .db
                .update_deploy_status(&rollback_id, "failed", Some(&e.to_string()))
                .await;
        }
    });

    Ok(Json(serde_json::json!({ "data": rollback_deploy })))
}

pub(super) async fn cancel_deploy(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(deploy_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deploy = state
        .db
        .get_deploy(&deploy_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;

    // Resolve the deploy's app and confirm it belongs to the caller's
    // team (member role); 404 if not, so cross-team deploys stay hidden.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &deploy.app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    match deploy.status.as_str() {
        // 'scheduled' deploys can be cancelled before their trigger time (IF-179).
        "scheduled" | "pending" | "building" | "deploying" => {}
        other => {
            return Err(ApiError::BadRequest(format!(
                "Cannot cancel deploy with status '{other}'"
            )));
        }
    }

    state
        .db
        .update_deploy_status(&deploy_id, "cancelled", Some("Cancelled by user"))
        .await?;

    state.event_bus.emit(
        crate::events::EventType::DeployStatus,
        Some(&deploy.app_id),
        Some(&deploy_id),
        serde_json::json!({"status": "cancelled"}),
    );

    let updated = state
        .db
        .get_deploy(&deploy_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;

    Ok(Json(serde_json::json!({ "data": updated })))
}

#[derive(Deserialize)]
pub(super) struct RescheduleRequest {
    pub scheduled_at: String,
}

/// Change the trigger time of a deploy that is still in the `scheduled` state
/// (IF-179). Rejects deploys that have already fired or are in any other state.
pub(super) async fn reschedule_deploy(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(deploy_id): Path<String>,
    Json(body): Json<RescheduleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deploy = state
        .db
        .get_deploy(&deploy_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;

    // The deploy's app must belong to the caller's team (member role); 404
    // otherwise so cross-team deploys stay hidden.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &deploy.app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    if deploy.status != "scheduled" {
        return Err(ApiError::BadRequest(format!(
            "Cannot reschedule deploy with status '{}'",
            deploy.status
        )));
    }

    let when = validate_scheduled_at(&body.scheduled_at)?;
    if !state.db.reschedule_deploy(&deploy_id, &when).await? {
        // Lost a race with the scheduler — the deploy just fired.
        return Err(ApiError::BadRequest("Deploy is no longer scheduled".into()));
    }

    state.event_bus.emit(
        crate::events::EventType::DeployStatus,
        Some(&deploy.app_id),
        Some(&deploy_id),
        serde_json::json!({ "status": "scheduled", "scheduled_at": when }),
    );

    let updated = state
        .db
        .get_deploy(&deploy_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;

    Ok(Json(serde_json::json!({ "data": updated })))
}

#[cfg(test)]
mod tests {
    use super::validate_scheduled_at;
    use crate::db::models::now_iso8601;

    /// The stored format must match `now_iso8601()` exactly (…Z, millis) so the
    /// scheduler's lexical `scheduled_at <= now` comparison stays correct.
    #[test]
    fn validated_format_matches_now_iso8601_shape() {
        let future = (chrono::Utc::now() + chrono::Duration::hours(1))
            .to_rfc3339_opts(chrono::SecondsFormat::Nanos, false); // +00:00, nanos
        let stored = validate_scheduled_at(&future).expect("future time is valid");
        let now = now_iso8601();

        // Same shape: ends with 'Z', millisecond precision (23 chars before 'Z').
        assert!(stored.ends_with('Z'), "stored should end with Z: {stored}");
        assert!(now.ends_with('Z'));
        assert_eq!(stored.len(), now.len(), "stored={stored} now={now}");
        assert_eq!(stored.find('.'), now.find('.'), "millis position differs");
    }

    #[test]
    fn past_time_is_rejected() {
        let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        assert!(validate_scheduled_at(&past).is_err());
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(validate_scheduled_at("not-a-date").is_err());
    }

    #[test]
    fn accepts_non_utc_offset_and_normalizes_to_utc() {
        // A time far in the future expressed in +02:00 must normalize to a Z time.
        let future = (chrono::Utc::now() + chrono::Duration::days(1))
            .with_timezone(&chrono::FixedOffset::east_opt(2 * 3600).unwrap());
        let stored = validate_scheduled_at(&future.to_rfc3339()).expect("valid future");
        assert!(stored.ends_with('Z'));
    }
}
