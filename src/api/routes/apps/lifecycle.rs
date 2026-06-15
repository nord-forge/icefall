use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::db::models::{App, UpdateApp};
use crate::deploy::raw_compose::RawComposeDeployer;

/// Build a raw-compose deployer for an app, or `None` when the app isn't in raw
/// compose mode. Raw stacks are owned by the `docker compose` CLI, so their
/// lifecycle goes through compose commands rather than per-container ops.
fn raw_compose(state: &AppState, app: &App) -> Option<RawComposeDeployer> {
    (app.deploy_mode == "raw-compose").then(|| {
        RawComposeDeployer::new(
            state.db.clone(),
            state.event_bus.clone(),
            state.config.clone(),
        )
    })
}

pub(super) async fn start_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // App must belong to the caller's team, member role to operate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    // Raw compose: `compose restart` is the closest "bring it back up" — there is
    // no compose "start" that recreates removed containers reliably across CLIs.
    if let Some(deployer) = raw_compose(&state, &app) {
        deployer.restart(&app).await.map_err(ApiError::internal)?;
        return Ok(Json(serde_json::json!({ "message": "started" })));
    }

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

pub(super) async fn stop_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // App must belong to the caller's team, member role to operate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    if let Some(deployer) = raw_compose(&state, &app) {
        deployer.stop(&app).await.map_err(ApiError::internal)?;
        return Ok(Json(serde_json::json!({ "message": "stopped" })));
    }

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

pub(super) async fn restart_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // App must belong to the caller's team, member role to operate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    if let Some(deployer) = raw_compose(&state, &app) {
        deployer.restart(&app).await.map_err(ApiError::internal)?;
        return Ok(Json(serde_json::json!({ "message": "restarted" })));
    }

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

pub(super) async fn wake_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // App must belong to the caller's team, member role to operate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    if app.ghost_mode_status != "hibernating" {
        return Ok(Json(
            serde_json::json!({ "message": "App is not hibernating", "status": app.ghost_mode_status }),
        ));
    }

    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;

    let mut started = 0u32;
    for container in &containers {
        if container.state != "running" {
            state.docker.start_container(&container.id).await?;
            started += 1;
        }
    }

    state
        .db
        .update_app(
            &id,
            &UpdateApp {
                ghost_mode_enabled: Some(app.ghost_mode_enabled),
                ..Default::default()
            },
        )
        .await?;

    Ok(Json(
        serde_json::json!({ "message": "waking", "containers": started }),
    ))
}
