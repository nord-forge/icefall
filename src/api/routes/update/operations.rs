use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use tracing::{error, info};

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::{new_id, now_iso8601, UpdateHistoryEntry};
use crate::update::apply::UpdateApplier;
use crate::update::discovery::UpdateChecker;
use crate::update::download::UpdateDownloader;
use crate::update::rollback::UpdateRollback;
use crate::update::CURRENT_VERSION;

use super::{require_admin, DEFAULT_GITHUB_REPO};

pub(super) async fn start_download(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let update_state = state.db.get_update_state().await?;

    let version = update_state
        .available_version
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("No update available to download".into()))?
        .to_string();

    if update_state.download_state == "downloading" {
        return Ok(Json(serde_json::json!({
            "data": {
                "status": "already_downloading",
                "version": version,
                "progress": update_state.download_progress,
            }
        })));
    }

    state
        .db
        .set_update_download_state("downloading", 0, None)
        .await?;

    let response_version = version.clone();

    let db = state.db.clone();
    let data_dir = state.config.data_dir.clone();

    tokio::spawn(async move {
        let updates_dir = data_dir.join("updates");
        let downloader = UpdateDownloader::new(updates_dir);

        let checker = UpdateChecker::new(DEFAULT_GITHUB_REPO);
        let check_state = match db.get_update_state().await {
            Ok(s) => s,
            Err(e) => {
                error!("failed to get update state for download: {e}");
                let _ = db.set_update_error(&e.to_string()).await;
                return;
            }
        };

        let result = checker
            .check_for_update(CURRENT_VERSION, &check_state.channel, "0.0.0")
            .await;

        let (manifest, _info) = match result {
            Ok(Some(pair)) => pair,
            Ok(None) => {
                let _ = db.set_update_error("Update no longer available").await;
                return;
            }
            Err(e) => {
                error!("re-check for download failed: {e}");
                let _ = db.set_update_error(&e.to_string()).await;
                return;
            }
        };

        let target = crate::update::artifact_target();
        let artifact = match manifest.artifact_for_target(target) {
            Some(a) => a.clone(),
            None => {
                let msg = format!("No artifact available for target {target}");
                error!("{msg}");
                let _ = db.set_update_error(&msg).await;
                return;
            }
        };

        let db_progress = db.clone();
        let download_version = version.clone();

        let result = downloader
            .download(&artifact, &download_version, |downloaded, total| {
                if total > 0 {
                    let pct = ((downloaded as f64 / total as f64) * 100.0) as i64;
                    let db_inner = db_progress.clone();
                    tokio::spawn(async move {
                        let _ = db_inner
                            .set_update_download_state("downloading", pct, None)
                            .await;
                    });
                }
            })
            .await;

        match result {
            Ok(path) => {
                let extract_result = downloader
                    .extract_and_validate(&path, &download_version)
                    .await;
                match extract_result {
                    Ok(binary_path) => {
                        let path_str = binary_path.to_string_lossy().to_string();
                        info!("download complete: {path_str}");
                        let _ = db
                            .set_update_download_state("ready", 100, Some(&path_str))
                            .await;
                    }
                    Err(e) => {
                        error!("extraction failed: {e}");
                        let _ = db.set_update_error(&e.to_string()).await;
                    }
                }
            }
            Err(e) => {
                error!("download failed: {e}");
                let _ = db.set_update_error(&e.to_string()).await;
            }
        }
    });

    Ok(Json(serde_json::json!({
        "data": {
            "status": "download_started",
            "version": response_version,
        }
    })))
}

pub(super) async fn apply_update(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let update_state = state.db.get_update_state().await?;

    if update_state.download_state != "ready" {
        return Err(ApiError::BadRequest(
            "Update must be downloaded before applying. Current state: ".to_string()
                + &update_state.download_state,
        ));
    }

    let version = update_state
        .available_version
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("No update version available".into()))?
        .to_string();

    let binary_path = update_state
        .download_path
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("No downloaded binary path found".into()))?
        .to_string();

    let data_dir = state.config.data_dir.clone();
    let db = state.db.clone();
    let from_version = CURRENT_VERSION.to_string();
    let to_version = version.clone();

    tokio::spawn(async move {
        let applier = UpdateApplier::new(&data_dir);
        let start = std::time::Instant::now();

        let result = applier
            .apply(
                std::path::Path::new(&binary_path),
                &from_version,
                &to_version,
                db.as_ref(),
                |step, status| {
                    info!("update apply: {step} = {status}");
                },
            )
            .await;

        let duration = start.elapsed().as_secs_f64();

        match result {
            Ok(()) => {
                info!("update applied successfully: {from_version} -> {to_version}");

                let entry = UpdateHistoryEntry {
                    id: new_id(),
                    version: to_version.clone(),
                    previous_version: from_version,
                    status: "success".to_string(),
                    duration_secs: Some(duration),
                    error: None,
                    changelog_url: None,
                    applied_at: now_iso8601(),
                };
                let _ = db.record_update_history(&entry).await;
                let _ = db.clear_update_available().await;
            }
            Err(e) => {
                error!("update apply failed: {e}");
                let _ = db.set_update_error(&e.to_string()).await;

                let entry = UpdateHistoryEntry {
                    id: new_id(),
                    version: to_version,
                    previous_version: from_version,
                    status: "failed".to_string(),
                    duration_secs: Some(duration),
                    error: Some(e.to_string()),
                    changelog_url: None,
                    applied_at: now_iso8601(),
                };
                let _ = db.record_update_history(&entry).await;
            }
        }
    });

    Ok(Json(serde_json::json!({
        "data": {
            "status": "applying",
            "version": version,
            "message": "Update is being applied. The service will restart shortly.",
        }
    })))
}

pub(super) async fn rollback_update(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let rollback = UpdateRollback::new(&state.config.data_dir);

    if !rollback.has_rollback() {
        return Err(ApiError::BadRequest("No rollback binary available".into()));
    }

    let info = rollback
        .rollback_info()
        .ok_or_else(|| ApiError::BadRequest("Could not read rollback binary metadata".into()))?;

    rollback
        .execute_rollback()
        .map_err(|e| ApiError::internal(std::io::Error::other(e.to_string())))?;

    let entry = UpdateHistoryEntry {
        id: new_id(),
        version: CURRENT_VERSION.to_string(),
        previous_version: CURRENT_VERSION.to_string(),
        status: "rolled_back".to_string(),
        duration_secs: None,
        error: None,
        changelog_url: None,
        applied_at: now_iso8601(),
    };
    let _ = state.db.record_update_history(&entry).await;

    Ok(Json(serde_json::json!({
        "data": {
            "status": "rolling_back",
            "rollback_binary": info.path,
            "message": "Rollback initiated. The service will restart shortly.",
        }
    })))
}

#[derive(Deserialize)]
pub(super) struct SkipVersionRequest {
    version: String,
}

pub(super) async fn skip_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SkipVersionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let version = body.version.trim().to_string();
    if version.is_empty() {
        return Err(ApiError::BadRequest("version is required".into()));
    }

    state.db.skip_update_version(&version).await?;

    let update_state = state.db.get_update_state().await?;
    if update_state.available_version.as_deref() == Some(&version) {
        state.db.clear_update_available().await?;
    }

    Ok(Json(serde_json::json!({
        "data": {
            "skipped": version,
        },
        "message": "Version skipped"
    })))
}
