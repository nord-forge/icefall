use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::build::orchestrator::BuildOrchestrator;
use crate::db::models::{NewDeploy, UpdateApp};
use crate::deploy::manager::DeployManager;

const MAX_INSTANCES: i64 = 20;
const VALID_LB_POLICIES: &[&str] = &["round_robin", "least_conn", "ip_hash", "random"];

#[derive(Debug, Deserialize)]
pub(super) struct ScaleRequest {
    desired_instances: i64,
}

/// `PUT /apps/{id}/scale` — set the desired instance count and trigger a
/// deploy to reach it. `desired_instances = 0` stops all instances.
pub(super) async fn scale_app(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ScaleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.desired_instances < 0 || body.desired_instances > MAX_INSTANCES {
        return Err(ApiError::BadRequest(format!(
            "desired_instances must be between 0 and {MAX_INSTANCES}"
        )));
    }

    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    state
        .db
        .update_app(
            &id,
            &UpdateApp {
                desired_instances: Some(body.desired_instances),
                ..Default::default()
            },
        )
        .await?;

    // Scaling to zero: tear down all running instances, no deploy needed.
    // Serialized per app via the build lock so it cannot race a scale-up.
    if body.desired_instances == 0 {
        let lock = state.build_locks.acquire(&id).await;
        let _guard = lock.lock().await;
        let manager = build_manager(&state);
        let instances = state.db.list_app_instances(&id).await?;
        for instance in &instances {
            let _ = manager.teardown_instance(instance).await;
        }
        return Ok(Json(serde_json::json!({
            "message": "scaled to zero",
            "desired_instances": 0,
        })));
    }

    let environments = state.db.list_environments(&id).await?;
    let env = environments
        .into_iter()
        .find(|e| e.env_type == "production")
        .ok_or_else(|| ApiError::BadRequest("app has no production environment".into()))?;

    let deploy = state
        .db
        .create_deploy(&NewDeploy {
            app_id: id.clone(),
            environment_id: env.id.clone(),
            git_sha: None,
            server_id: None,
            tag: None,
            no_cache: false,
        })
        .await?;
    let deploy_id = deploy.id.clone();
    let response_deploy_id = deploy_id.clone();

    // Build once on the control plane, then distribute across instances.
    // Serialized per app via the build lock so concurrent scale/deploy
    // operations on the same app cannot interleave.
    tokio::spawn(async move {
        let lock = state.build_locks.acquire(&id).await;
        let _guard = lock.lock().await;

        let manager = build_manager(&state);
        let Ok(Some(app)) = state.db.get_app(&id).await else {
            return;
        };

        let orchestrator =
            BuildOrchestrator::new(state.docker.clone(), state.db.clone(), state.config.clone());
        let image_ref = match orchestrator.build(&deploy_id, &app, None, false).await {
            Ok(result) => result.image_ref,
            Err(e) => {
                tracing::error!("Scale build failed for app {}: {e}", app.name);
                let _ = state
                    .db
                    .update_deploy_status(&deploy_id, "failed", Some(&e.to_string()))
                    .await;
                return;
            }
        };

        if let Err(e) = manager
            .deploy_instances(&deploy, &app, &env, &image_ref)
            .await
        {
            tracing::error!("Scale deploy failed for app {}: {e}", app.name);
        }
    });

    Ok(Json(serde_json::json!({
        "message": "scaling",
        "deploy_id": response_deploy_id,
        "desired_instances": body.desired_instances,
    })))
}

/// `GET /apps/{id}/instances` — list all instances with server, status, port.
pub(super) async fn list_instances(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let instances = state.db.list_app_instances(&id).await?;
    Ok(Json(serde_json::json!({ "data": instances })))
}

#[derive(Debug, Deserialize)]
pub(super) struct LbConfigRequest {
    policy: Option<String>,
    health_check_path: Option<String>,
    sticky_sessions: Option<bool>,
}

/// `PUT /apps/{id}/lb-config` — update load balancing policy, health check
/// path, and sticky-session setting. Also re-applies the Caddy route so the
/// change takes effect immediately for a running multi-instance app.
pub(super) async fn update_lb_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<LbConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    if let Some(ref policy) = body.policy {
        if !VALID_LB_POLICIES.contains(&policy.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "invalid lb policy '{policy}'; expected one of {VALID_LB_POLICIES:?}"
            )));
        }
    }
    if let Some(ref path) = body.health_check_path {
        if !path.starts_with('/') {
            return Err(ApiError::BadRequest(
                "health_check_path must start with '/'".into(),
            ));
        }
    }

    state
        .db
        .update_app(
            &id,
            &UpdateApp {
                lb_policy: body.policy,
                lb_health_check_path: body.health_check_path,
                lb_sticky_sessions: body.sticky_sessions,
                ..Default::default()
            },
        )
        .await?;

    // Re-apply Caddy routing so the new policy/health path takes effect now.
    if let Ok(Some(app)) = state.db.get_app(&id).await {
        if let Ok(envs) = state.db.list_environments(&id).await {
            if let Some(env) = envs.into_iter().find(|e| e.env_type == "production") {
                let manager = build_manager(&state);
                if let Err(e) = manager.rebuild_caddy_routes(&app, &env).await {
                    tracing::warn!("Failed to re-apply Caddy routes for app {id}: {e}");
                }
            }
        }
    }

    Ok(Json(serde_json::json!({ "message": "lb config updated" })))
}

/// `DELETE /apps/{id}/instances/{instance_id}` — remove one instance.
pub(super) async fn delete_instance(
    State(state): State<AppState>,
    Path((id, instance_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let instance = state
        .db
        .get_app_instance(&instance_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Instance '{instance_id}' not found")))?;

    if instance.app_id != id {
        return Err(ApiError::BadRequest(
            "instance does not belong to this app".into(),
        ));
    }

    let manager = build_manager(&state);
    manager
        .teardown_instance(&instance)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    Ok(Json(serde_json::json!({ "message": "instance removed" })))
}

fn build_manager(state: &AppState) -> DeployManager {
    DeployManager::new(
        state.docker.clone(),
        state.caddy.clone(),
        state.db.clone(),
        state.config.clone(),
        state.event_bus.clone(),
        Some(state.agent_registry.clone()),
    )
}
