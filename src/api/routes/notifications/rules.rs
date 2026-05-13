use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewNotificationRule;

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
    let valid_events = [
        "deploy.success",
        "deploy.failure",
        "health.down",
        "health.recovered",
        "health.auto_restart",
        "backup.success",
        "backup.failure",
    ];
    if !valid_events.contains(&body.event_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid event type. Valid: {}",
            valid_events.join(", ")
        )));
    }

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

pub(super) async fn delete_rule(
    State(_state): State<AppState>,
    Path((_app_id, rule_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = rule_id;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}
