use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::db::models::UpdateApp;

pub(super) async fn start_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H6: app must belong to the caller's team, member role to operate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

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
    // H6: app must belong to the caller's team, member role to operate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

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
    // H6: app must belong to the caller's team, member role to operate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

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
    // H6: app must belong to the caller's team, member role to operate.
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
