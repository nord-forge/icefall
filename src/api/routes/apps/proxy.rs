//! IF-149 reverse proxy management endpoints. App-scoped config read/toggle/
//! advanced/reset/undo; viewer reads, member toggles presets, admin advanced+reset.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::caddy::proxy::ProxyPresets;
use crate::db::models::App;

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

    let mut presets = presets;
    resolve_basic_auth_password(&mut presets, &app)?;

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

    // The custom config is this app's route object(s) — reject anything else.
    if !body.config.is_object() && !body.config.is_array() {
        return Err(ApiError::BadRequest(
            "Custom proxy config must be a Caddy route object or array of routes".into(),
        ));
    }

    // Validate by wrapping the routes in a throwaway full config — Caddy's
    // validator only accepts a complete config; never touches live config.
    let probe = wrap_routes_for_validation(&body.config);
    state
        .caddy
        .validate_config(&probe)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Caddy rejected the config: {e}")))?;

    let serialized = serde_json::to_string(&body.config).unwrap_or_default();

    snapshot_current_config(&state, &app_id).await;
    state
        .db
        .set_custom_proxy_config(&app_id, &serialized)
        .await?;

    // Apply scoped to THIS app's domains only — never replace the whole server
    // config (which would clobber other apps/teams).
    let domains = app_domains(&state, &app_id).await;
    state
        .caddy
        .apply_scoped_routes(&domains, &body.config)
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

    if !body.config.is_object() && !body.config.is_array() {
        return Ok(Json(serde_json::json!({
            "data": { "valid": false, "error": "Config must be a route object or array of routes" }
        })));
    }

    let probe = wrap_routes_for_validation(&body.config);
    match state.caddy.validate_config(&probe).await {
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

/// POST /apps/{id}/proxy/undo — restore the most recent full-config snapshot.
/// Admin only: re-applies captured server config (same blast radius as apply).
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
    ctx.verify_team_access(&app.team_id, TeamRole::Admin)?;

    let snapshot = state
        .db
        .latest_proxy_config_history(&app_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("No previous config to restore".into()))?;

    let config: serde_json::Value =
        serde_json::from_str(&snapshot.config).map_err(|e| ApiError::Internal(Box::new(e)))?;

    // The snapshot is a full Caddy config captured before the last change;
    // restore it atomically. Caddy keeps the current config if this is rejected.
    if config.is_object() {
        state
            .caddy
            .load_config(&config)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to restore config: {e}")))?;
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

/// Capture the current full Caddy config as a rollback point. Best-effort; the
/// full config (not just routes) is stored so `undo` can re-apply it via `/load`.
async fn snapshot_current_config(state: &AppState, app_id: &str) {
    if let Ok(config) = state.caddy.get_full_config().await {
        let serialized = config.to_string();
        if let Err(e) = state
            .db
            .record_proxy_config_history(app_id, &serialized)
            .await
        {
            tracing::warn!(error = %e, app_id, "failed to snapshot proxy config history");
        }
    }
}

/// The app's domains, used to scope route operations. Best-effort: an empty list
/// means we only append (no existing routes to remove).
async fn app_domains(state: &AppState, app_id: &str) -> Vec<String> {
    state
        .db
        .list_domains(app_id)
        .await
        .map(|ds| ds.into_iter().map(|d| d.domain).collect())
        .unwrap_or_default()
}

/// Wrap a user's route object/array in a minimal full Caddy config for the
/// `/load` validator (which only accepts a complete config). Validation only.
fn wrap_routes_for_validation(routes: &serde_json::Value) -> serde_json::Value {
    let routes_array = match routes {
        serde_json::Value::Array(_) => routes.clone(),
        other => serde_json::Value::Array(vec![other.clone()]),
    };
    serde_json::json!({
        "apps": { "http": { "servers": { "_validate": {
            "listen": [":0"],
            "routes": routes_array,
        }}}}
    })
}

/// Resolve the basic-auth preset's plaintext `password` into a stored bcrypt
/// `password_hash`, clearing the plaintext. Empty password keeps the existing hash.
fn resolve_basic_auth_password(presets: &mut ProxyPresets, app: &App) -> Result<(), ApiError> {
    let existing_hash = app
        .proxy_presets
        .as_deref()
        .and_then(|s| serde_json::from_str::<ProxyPresets>(s).ok())
        .and_then(|p| p.basic_auth)
        .map(|b| b.password_hash);
    resolve_basic_auth_password_inner(presets, existing_hash.as_deref())
}

/// Pure core of [`resolve_basic_auth_password`], split out for testing without an
/// `App`. `existing_hash` is the app's currently-stored basic-auth hash, if any.
fn resolve_basic_auth_password_inner(
    presets: &mut ProxyPresets,
    existing_hash: Option<&str>,
) -> Result<(), ApiError> {
    let Some(ba) = presets.basic_auth.as_mut() else {
        return Ok(());
    };

    let provided = ba.password.take().unwrap_or_default();
    if !provided.is_empty() {
        ba.password_hash = bcrypt::hash(&provided, bcrypt::DEFAULT_COST)
            .map_err(|e| ApiError::Internal(Box::new(e)))?;
    } else if ba.password_hash.is_empty() {
        // No new password supplied — reuse the existing stored hash.
        let existing = existing_hash.unwrap_or_default();
        if existing.is_empty() && ba.enabled {
            return Err(ApiError::BadRequest(
                "Basic auth is enabled but no password is set. Provide a password.".into(),
            ));
        }
        ba.password_hash = existing.to_string();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caddy::proxy::BasicAuthPreset;

    fn presets_with_auth(password: Option<&str>, hash: &str) -> ProxyPresets {
        ProxyPresets {
            basic_auth: Some(BasicAuthPreset {
                enabled: true,
                username: "u".into(),
                password_hash: hash.into(),
                password: password.map(|s| s.to_string()),
                path: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn wrap_routes_wraps_single_object_in_array() {
        let route = serde_json::json!({ "handle": [] });
        let wrapped = wrap_routes_for_validation(&route);
        let routes = &wrapped["apps"]["http"]["servers"]["_validate"]["routes"];
        assert!(routes.is_array());
        assert_eq!(routes.as_array().unwrap().len(), 1);
    }

    #[test]
    fn wrap_routes_passes_array_through() {
        let arr = serde_json::json!([{ "a": 1 }, { "b": 2 }]);
        let wrapped = wrap_routes_for_validation(&arr);
        let routes = &wrapped["apps"]["http"]["servers"]["_validate"]["routes"];
        assert_eq!(routes.as_array().unwrap().len(), 2);
    }

    #[test]
    fn new_password_is_bcrypt_hashed_and_plaintext_cleared() {
        let mut presets = presets_with_auth(Some("hunter2"), "");
        resolve_basic_auth_password_inner(&mut presets, None).unwrap();
        let ba = presets.basic_auth.unwrap();
        assert!(
            ba.password_hash.starts_with("$2"),
            "should be a bcrypt hash"
        );
        assert!(bcrypt::verify("hunter2", &ba.password_hash).unwrap());
        assert!(ba.password.is_none(), "plaintext must be cleared");
    }

    #[test]
    fn blank_password_keeps_existing_hash() {
        let mut presets = presets_with_auth(None, "");
        resolve_basic_auth_password_inner(&mut presets, Some("$2b$existinghash")).unwrap();
        assert_eq!(
            presets.basic_auth.unwrap().password_hash,
            "$2b$existinghash"
        );
    }

    #[test]
    fn enabled_auth_with_no_password_anywhere_errors() {
        let mut presets = presets_with_auth(None, "");
        let err = resolve_basic_auth_password_inner(&mut presets, None);
        assert!(err.is_err(), "enabling auth without any password must fail");
    }

    #[test]
    fn no_basic_auth_preset_is_a_noop() {
        let mut presets = ProxyPresets::default();
        assert!(resolve_basic_auth_password_inner(&mut presets, None).is_ok());
        assert!(presets.basic_auth.is_none());
    }
}
