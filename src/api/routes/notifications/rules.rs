use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewNotificationRule;

/// App-scoped events (subscribed per app).
const APP_EVENTS: &[&str] = &[
    "deploy.success",
    "deploy.failure",
    // Scheduled-deploy lifecycle (IF-179).
    "deploy.scheduled",
    "deploy.started",
    "deploy.missed",
    "health.down",
    "health.recovered",
    "health.auto_restart",
];

/// System/global events (server reachability, disk, backups) — subscribed via
/// the global-rules endpoint, scoped to "*" (IF-167).
const SYSTEM_EVENTS: &[&str] = &[
    "server.online",
    "server.offline",
    "server.disk.warning",
    "server.disk.critical",
    "server.disk.recovered",
    "backup.success",
    "backup.failure",
];

fn validate_event(event_type: &str, allowed: &[&str]) -> Result<(), ApiError> {
    if allowed.contains(&event_type) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "Invalid event type. Valid: {}",
            allowed.join(", ")
        )))
    }
}

pub(super) async fn list_rules(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rules = state.db.get_notification_rules(&app_id).await?;
    Ok(Json(serde_json::json!({ "data": rules })))
}

#[derive(Deserialize)]
pub(super) struct CreateRuleRequest {
    notification_id: String,
    event_type: String,
}

pub(super) async fn create_rule(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
    Json(body): Json<CreateRuleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // App rules may subscribe to app events or system events (a per-app
    // catch-all); global-only events use the global-rules endpoint.
    validate_event(&body.event_type, &[APP_EVENTS, SYSTEM_EVENTS].concat())?;

    let rule = state
        .db
        .create_notification_rule(&NewNotificationRule {
            app_id,
            notification_id: body.notification_id,
            event_type: body.event_type,
        })
        .await?;

    Ok(Json(serde_json::json!({ "data": rule })))
}

/// List the global (system-wide) notification rules (IF-167).
pub(super) async fn list_global_rules(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rules = state
        .db
        .get_notification_rules(super::emit::GLOBAL_SCOPE)
        .await?;
    Ok(Json(serde_json::json!({
        "data": rules,
        "available_events": SYSTEM_EVENTS,
    })))
}

/// Subscribe a channel to a system-wide event (server/disk/backup) (IF-167).
pub(super) async fn create_global_rule(
    State(state): State<AppState>,
    Json(body): Json<CreateRuleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validate_event(&body.event_type, SYSTEM_EVENTS)?;

    let rule = state
        .db
        .create_notification_rule(&NewNotificationRule {
            app_id: super::emit::GLOBAL_SCOPE.to_string(),
            notification_id: body.notification_id,
            event_type: body.event_type,
        })
        .await?;

    Ok(Json(serde_json::json!({ "data": rule })))
}

pub(super) async fn delete_rule(
    State(state): State<AppState>,
    Path((_app_id, rule_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.delete_notification_rule(&rule_id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}
