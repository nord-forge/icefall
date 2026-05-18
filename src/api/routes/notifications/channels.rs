use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewNotification;

use super::dispatch::dispatch_notification;

pub(super) async fn list_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let channels = state.db.list_notification_channels().await?;
    let safe: Vec<serde_json::Value> = channels
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "channel_type": c.channel_type,
                "config": "••••••••",
                "created_at": c.created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": safe })))
}

#[derive(Deserialize)]
pub(super) struct CreateChannelRequest {
    channel_type: String,
    config: serde_json::Value,
}

pub(super) async fn create_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateChannelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    if !["smtp", "webhook", "ntfy", "plunk", "slack", "discord"]
        .contains(&body.channel_type.as_str())
    {
        return Err(ApiError::BadRequest(
            "channel_type must be smtp, webhook, ntfy, plunk, slack, or discord".into(),
        ));
    }

    let config_str = serde_json::to_string(&body.config).unwrap_or_default();
    let channel = state
        .db
        .create_notification_channel(&NewNotification {
            channel_type: body.channel_type,
            config: config_str,
        })
        .await?;

    Ok(Json(
        serde_json::json!({ "data": { "id": channel.id, "channel_type": channel.channel_type } }),
    ))
}

pub(super) async fn delete_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let _ = id;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

pub(super) async fn test_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let channels = state.db.list_notification_channels().await?;
    let channel = channels
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("channel {id}")))?;

    match dispatch_notification(
        &channel.channel_type,
        &channel.config,
        "test",
        "Test notification from Icefall",
        &serde_json::json!({"message": "If you see this, notifications are working!"}),
    )
    .await
    {
        Ok(()) => Ok(Json(
            serde_json::json!({ "message": "test notification sent" }),
        )),
        Err(e) => Ok(Json(
            serde_json::json!({ "message": format!("test failed: {e}") }),
        )),
    }
}
