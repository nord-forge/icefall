use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::update::CURRENT_VERSION;

use super::require_admin;

pub(super) async fn get_update_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let update_state = state.db.get_update_state().await?;

    Ok(Json(serde_json::json!({
        "data": {
            "channel": update_state.channel,
            "current_version": CURRENT_VERSION,
            "auto_update_enabled": update_state.auto_update_enabled,
            "auto_update_channel": update_state.auto_update_channel,
            "auto_update_window_start": update_state.auto_update_window_start,
            "auto_update_window_end": update_state.auto_update_window_end,
            "auto_update_notify_before_minutes": update_state.auto_update_notify_before_minutes,
        }
    })))
}

#[derive(Deserialize)]
pub(super) struct UpdatePreferencesRequest {
    channel: Option<String>,
    auto_update_enabled: Option<bool>,
    auto_update_channel: Option<String>,
    auto_update_window_start: Option<String>,
    auto_update_window_end: Option<String>,
    auto_update_notify_before_minutes: Option<i64>,
}

pub(super) async fn update_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdatePreferencesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let valid_channels = ["stable", "beta", "nightly"];

    if let Some(ref channel) = body.channel {
        if !valid_channels.contains(&channel.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Invalid channel '{}'. Must be one of: {}",
                channel,
                valid_channels.join(", ")
            )));
        }
        state.db.set_update_channel(channel).await?;
    }

    if body.auto_update_enabled.is_some()
        || body.auto_update_channel.is_some()
        || body.auto_update_window_start.is_some()
        || body.auto_update_window_end.is_some()
        || body.auto_update_notify_before_minutes.is_some()
    {
        let current = state.db.get_update_state().await?;

        let enabled = body
            .auto_update_enabled
            .unwrap_or(current.auto_update_enabled);
        let au_channel = body
            .auto_update_channel
            .as_deref()
            .unwrap_or(&current.auto_update_channel);

        if !valid_channels.contains(&au_channel) {
            return Err(ApiError::BadRequest(format!(
                "Invalid auto_update_channel '{}'. Must be one of: {}",
                au_channel,
                valid_channels.join(", ")
            )));
        }

        let window_start = body
            .auto_update_window_start
            .as_deref()
            .unwrap_or(&current.auto_update_window_start);
        let window_end = body
            .auto_update_window_end
            .as_deref()
            .unwrap_or(&current.auto_update_window_end);

        if chrono::NaiveTime::parse_from_str(window_start, "%H:%M").is_err() {
            return Err(ApiError::BadRequest(format!(
                "Invalid window_start '{}'. Use HH:MM format.",
                window_start
            )));
        }
        if chrono::NaiveTime::parse_from_str(window_end, "%H:%M").is_err() {
            return Err(ApiError::BadRequest(format!(
                "Invalid window_end '{}'. Use HH:MM format.",
                window_end
            )));
        }

        let notify_before = body
            .auto_update_notify_before_minutes
            .unwrap_or(current.auto_update_notify_before_minutes);

        state
            .db
            .set_auto_update_settings(enabled, au_channel, window_start, window_end, notify_before)
            .await?;
    }

    let update_state = state.db.get_update_state().await?;

    Ok(Json(serde_json::json!({
        "data": {
            "channel": update_state.channel,
            "current_version": CURRENT_VERSION,
            "auto_update_enabled": update_state.auto_update_enabled,
            "auto_update_channel": update_state.auto_update_channel,
            "auto_update_window_start": update_state.auto_update_window_start,
            "auto_update_window_end": update_state.auto_update_window_end,
            "auto_update_notify_before_minutes": update_state.auto_update_notify_before_minutes,
        },
        "message": "Update preferences saved"
    })))
}
