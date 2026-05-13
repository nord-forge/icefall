use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::now_iso8601;
use crate::update::discovery::UpdateChecker;
use crate::update::rollback::UpdateRollback;
use crate::update::CURRENT_VERSION;

use super::{require_admin, DEFAULT_GITHUB_REPO};

pub(super) async fn check_for_update(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let update_state = state.db.get_update_state().await?;

    let now = now_iso8601();
    state.db.set_last_check_at(&now).await?;

    let checker = UpdateChecker::new(DEFAULT_GITHUB_REPO);
    let result = checker
        .check_for_update(
            CURRENT_VERSION,
            &update_state.channel,
            &update_state.highest_seen_version,
        )
        .await;

    match result {
        Ok(Some((manifest, info))) => {
            let skipped = state.db.is_version_skipped(&info.version).await?;
            if skipped {
                return Ok(Json(serde_json::json!({
                    "data": {
                        "available": false,
                        "current_version": CURRENT_VERSION,
                        "latest_version": info.version,
                        "skipped": true,
                        "checked_at": now,
                    }
                })));
            }

            let highlights_json =
                serde_json::to_string(&info.changelog_highlights).unwrap_or_default();
            state
                .db
                .set_update_available(
                    &info.version,
                    &info.release_url,
                    &info.release_notes,
                    &highlights_json,
                )
                .await?;
            state.db.update_highest_seen(&info.version).await?;

            Ok(Json(serde_json::json!({
                "data": {
                    "available": true,
                    "current_version": CURRENT_VERSION,
                    "latest_version": info.version,
                    "changelog_highlights": info.changelog_highlights,
                    "changelog_url": manifest.changelog_url,
                    "breaking": manifest.breaking,
                    "breaking_changes": manifest.breaking_changes,
                    "published_at": info.published_at,
                    "checked_at": now,
                }
            })))
        }
        Ok(None) => Ok(Json(serde_json::json!({
            "data": {
                "available": false,
                "current_version": CURRENT_VERSION,
                "latest_version": CURRENT_VERSION,
                "checked_at": now,
            }
        }))),
        Err(e) => {
            let _ = state.db.set_update_error(&e.to_string()).await;
            Err(ApiError::internal(std::io::Error::other(e.to_string())))
        }
    }
}

pub(super) async fn get_update_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let update_state = state.db.get_update_state().await?;

    let highlights: Vec<String> = update_state
        .changelog_highlights
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let rollback = UpdateRollback::new(&state.config.data_dir);
    let rollback_available = rollback.has_rollback();

    Ok(Json(serde_json::json!({
        "data": {
            "current_version": CURRENT_VERSION,
            "available_version": update_state.available_version,
            "release_url": update_state.release_url,
            "changelog_highlights": highlights,
            "channel": update_state.channel,
            "download_state": update_state.download_state,
            "download_progress": update_state.download_progress,
            "last_check_at": update_state.last_check_at,
            "last_update_at": update_state.last_update_at,
            "last_update_version": update_state.last_update_version,
            "error_message": update_state.error_message,
            "rollback_available": rollback_available,
        }
    })))
}

pub(super) async fn get_update_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let history = state.db.list_update_history(50).await?;

    let entries: Vec<serde_json::Value> = history
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "version": e.version,
                "previous_version": e.previous_version,
                "status": e.status,
                "duration_secs": e.duration_secs,
                "error": e.error,
                "changelog_url": e.changelog_url,
                "applied_at": e.applied_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "data": entries
    })))
}
