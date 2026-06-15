//! IF-149 reverse proxy management endpoints.
//!
//! App-scoped: read config, toggle presets, advanced raw config (validate/apply),
//! reset to auto-generated, and undo the last change from history. Role policy:
//! viewer reads, member toggles presets, admin uses advanced mode + reset.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::caddy::proxy::ProxyPresets;

/// GET /apps/{id}/proxy — current proxy state for the read-only viewer.
pub(super) async fn get_proxy(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;

    // Live routes from Caddy for this app's domains, best-effort — Caddy may be
    // briefly unreachable without making the config inspectable.
    let routes = state
        .caddy
        .get_routes_config()
        .await
        .unwrap_or(serde_json::Value::Array(vec![]));

    let presets: ProxyPresets = app
        .proxy_presets
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "data": {
            "has_custom_proxy_config": app.has_custom_proxy_config,
            "custom_proxy_config": app.custom_proxy_config,
            "presets": presets,
            "routes": routes,
        }
    })))
}

/// PUT /apps/{id}/proxy/presets — replace the app's middleware presets.
/// Disabled while the app is in advanced mode (raw config takes precedence).
pub(super) async fn update_presets(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
    Json(presets): Json<ProxyPresets>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    if app.has_custom_proxy_config {
        return Err(ApiError::BadRequest(
            "App is in advanced mode; disable it (reset) before editing presets".into(),
        ));
    }

    let serialized = serde_json::to_string(&presets)
        .map_err(|e| ApiError::BadRequest(format!("invalid presets: {e}")))?;

    // Snapshot current routes before changing anything, for rollback.
    snapshot_current_config(&state, &app_id).await;

    state.db.set_proxy_presets(&app_id, &serialized).await?;

    Ok(Json(serde_json::json!({
        "data": { "presets": presets },
        "message": "Presets updated. They apply on the next deploy or config regeneration."
    })))
}

#[derive(Deserialize)]
pub(super) struct CustomConfigRequest {
    config: serde_json::Value,
}

/// PUT /apps/{id}/proxy/custom — save raw config and enter advanced mode.
/// Validated against Caddy before persisting; admin only.
pub(super) async fn set_custom(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
    Json(body): Json<CustomConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Admin)?;

    // Reject non-object configs early with a clear message.
    if !body.config.is_object() {
        return Err(ApiError::BadRequest(
            "Proxy config must be a JSON object".into(),
        ));
    }

    // Validate without applying — Caddy returns a descriptive error on bad config.
    state
        .caddy
        .validate_config(&body.config)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Caddy rejected the config: {e}")))?;

    let serialized = serde_json::to_string(&body.config).unwrap_or_default();

    snapshot_current_config(&state, &app_id).await;
    state
        .db
        .set_custom_proxy_config(&app_id, &serialized)
        .await?;

    // Apply it now so the user sees the effect immediately.
    state
        .caddy
        .load_config(&body.config)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to apply config: {e}")))?;

    Ok(Json(serde_json::json!({
        "data": { "has_custom_proxy_config": true },
        "message": "Custom proxy config applied."
    })))
}

/// POST /apps/{id}/proxy/validate — validate a config without applying. Open to
/// members so they can check edits before an admin applies them.
pub(super) async fn validate(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
    Json(body): Json<CustomConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    match state.caddy.validate_config(&body.config).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "data": { "valid": true }
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "data": { "valid": false, "error": e.to_string() }
        }))),
    }
}

/// POST /apps/{id}/proxy/reset — discard custom config and return to
/// preset/auto-generated mode. Admin only.
pub(super) async fn reset(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Admin)?;

    snapshot_current_config(&state, &app_id).await;
    state.db.clear_custom_proxy_config(&app_id).await?;

    Ok(Json(serde_json::json!({
        "data": { "has_custom_proxy_config": false },
        "message": "Reset to auto-generated config. It regenerates on the next deploy."
    })))
}

/// POST /apps/{id}/proxy/undo — restore the most recent config snapshot. Member.
pub(super) async fn undo(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    let snapshot = state
        .db
        .latest_proxy_config_history(&app_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("No previous config to restore".into()))?;

    let config: serde_json::Value =
        serde_json::from_str(&snapshot.config).map_err(|e| ApiError::Internal(Box::new(e)))?;

    // Re-apply the snapshot to Caddy when it was a full custom config.
    if config.is_object() {
        let _ = state.caddy.load_config(&config).await;
    }

    Ok(Json(serde_json::json!({
        "data": { "restored_at": snapshot.created_at },
        "message": "Restored the previous proxy config."
    })))
}

/// GET /apps/{id}/proxy/history — list rollback snapshots (viewer).
pub(super) async fn history(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;

    let entries = state.db.list_proxy_config_history(&app_id).await?;
    Ok(Json(serde_json::json!({ "data": entries })))
}

/// Capture the current live Caddy route config as a rollback point. Best-effort:
/// a snapshot failure must not block the actual config change.
async fn snapshot_current_config(state: &AppState, app_id: &str) {
    if let Ok(routes) = state.caddy.get_routes_config().await {
        let serialized = routes.to_string();
        if let Err(e) = state
            .db
            .record_proxy_config_history(app_id, &serialized)
            .await
        {
            tracing::warn!(error = %e, app_id, "failed to snapshot proxy config history");
        }
    }
}
