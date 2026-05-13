use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::config::IcefallConfig;
use crate::db::models::now_iso8601;
use crate::db::Database;
use crate::events::{EventBus, EventType};
use crate::update::apply::UpdateApplier;
use crate::update::discovery::UpdateChecker;
use crate::update::download::UpdateDownloader;
use crate::update::CURRENT_VERSION;

const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);
const WINDOW_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);
const DEPLOY_WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

const DEFAULT_GITHUB_REPO: &str = "nordforge/icefall";

pub fn spawn_update_checker(
    db: Arc<dyn Database>,
    config: Arc<IcefallConfig>,
    event_bus: Arc<EventBus>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        let checker = UpdateChecker::new(DEFAULT_GITHUB_REPO);

        loop {
            debug!("running periodic update check");
            if let Err(e) = run_check(&checker, &db, &event_bus).await {
                warn!("update check failed: {e}");
            }

            // After checking, attempt auto-update pre-download
            if let Err(e) = try_pre_download(&checker, &db, &config).await {
                debug!("pre-download skipped or failed: {e}");
            }

            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

/// Spawn the maintenance window task that applies pre-downloaded updates.
pub fn spawn_auto_update_applier(
    db: Arc<dyn Database>,
    config: Arc<IcefallConfig>,
    event_bus: Arc<EventBus>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        loop {
            if let Err(e) = try_auto_apply(&db, &config, &event_bus).await {
                debug!("auto-apply skipped: {e}");
            }
            tokio::time::sleep(WINDOW_POLL_INTERVAL).await;
        }
    });
}

async fn run_check(
    checker: &UpdateChecker,
    db: &Arc<dyn Database>,
    event_bus: &Arc<EventBus>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = now_iso8601();
    db.set_last_check_at(&now).await.map_err(|e| {
        error!("failed to set last_check_at: {e}");
        e
    })?;

    let state = db.get_update_state().await?;

    let result = checker
        .check_for_update(CURRENT_VERSION, &state.channel, &state.highest_seen_version)
        .await;

    match result {
        Ok(Some((manifest, info))) => {
            if db.is_version_skipped(&info.version).await? {
                debug!("version {} is skipped, ignoring", info.version);
                return Ok(());
            }

            info!(
                "new update available: {} (current: {CURRENT_VERSION})",
                info.version
            );

            let highlights_json = serde_json::to_string(&info.changelog_highlights)?;
            db.set_update_available(
                &info.version,
                &info.release_url,
                &info.release_notes,
                &highlights_json,
            )
            .await?;

            db.update_highest_seen(&info.version).await?;

            event_bus.emit(
                EventType::UpdateAvailable,
                None,
                None,
                serde_json::json!({
                    "version": info.version,
                    "release_url": info.release_url,
                    "breaking": manifest.breaking,
                    "requires_migration": manifest.requires_migration,
                    "changelog_highlights": info.changelog_highlights,
                }),
            );
        }
        Ok(None) => {
            debug!("no update available (current: {CURRENT_VERSION})");
        }
        Err(e) => {
            warn!("update check error: {e}");
            let _ = db.set_update_error(&e.to_string()).await;
        }
    }

    Ok(())
}

/// Pre-download the update artifact when auto-update is enabled and a new
/// version has been discovered.  Runs outside the maintenance window so the
/// apply step is fast.
async fn try_pre_download(
    checker: &UpdateChecker,
    db: &Arc<dyn Database>,
    config: &Arc<IcefallConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = db.get_update_state().await?;

    if !state.auto_update_enabled {
        return Ok(());
    }

    if state.available_version.is_none() {
        return Ok(());
    }

    // Already downloaded or downloading
    if state.download_state == "ready" || state.download_state == "downloading" {
        return Ok(());
    }

    let version = state.available_version.as_deref().unwrap();
    info!(version, "auto-update: starting pre-download");

    db.set_update_download_state("downloading", 0, None).await?;

    let check_state = db.get_update_state().await?;
    let result = checker
        .check_for_update(CURRENT_VERSION, &check_state.channel, "0.0.0")
        .await;

    let (manifest, _info) = match result {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            let _ = db.set_update_error("Update no longer available").await;
            return Ok(());
        }
        Err(e) => {
            let _ = db.set_update_error(&e.to_string()).await;
            return Err(e.into());
        }
    };

    // Skip pre-download for breaking changes
    if manifest.breaking {
        info!("auto-update: skipping pre-download for breaking change");
        db.set_update_download_state("none", 0, None).await?;
        return Ok(());
    }

    let target = crate::update::artifact_target();
    let artifact = match manifest.artifact_for_target(target) {
        Some(a) => a.clone(),
        None => {
            let msg = format!("No artifact for target {target}");
            let _ = db.set_update_error(&msg).await;
            return Err(msg.into());
        }
    };

    let updates_dir = config.data_dir.join("updates");
    let downloader = UpdateDownloader::new(updates_dir);

    let db_progress = db.clone();
    let last_reported_pct = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(-1));
    let download_result = downloader
        .download(&artifact, version, |downloaded, total| {
            if total > 0 {
                let pct = ((downloaded as f64 / total as f64) * 100.0) as i64;
                let last = last_reported_pct.load(std::sync::atomic::Ordering::Relaxed);
                if pct >= last + 5 || pct == 100 {
                    last_reported_pct.store(pct, std::sync::atomic::Ordering::Relaxed);
                    let db_inner = db_progress.clone();
                    tokio::spawn(async move {
                        let _ = db_inner
                            .set_update_download_state("downloading", pct, None)
                            .await;
                    });
                }
            }
        })
        .await;

    match download_result {
        Ok(path) => {
            let extract_result = downloader.extract_and_validate(&path, version).await;
            match extract_result {
                Ok(binary_path) => {
                    let path_str = binary_path.to_string_lossy().to_string();
                    info!("auto-update: pre-download complete: {path_str}");
                    db.set_update_download_state("ready", 100, Some(&path_str))
                        .await?;
                    db.set_auto_update_pre_downloaded(true).await?;
                }
                Err(e) => {
                    error!("auto-update: extraction failed: {e}");
                    let _ = db.set_update_error(&e.to_string()).await;
                }
            }
        }
        Err(e) => {
            error!("auto-update: download failed: {e}");
            let _ = db.set_update_error(&e.to_string()).await;
        }
    }

    Ok(())
}

/// Check if we're inside the maintenance window and should apply an update.
async fn try_auto_apply(
    db: &Arc<dyn Database>,
    config: &Arc<IcefallConfig>,
    event_bus: &Arc<EventBus>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = db.get_update_state().await?;

    if !state.auto_update_enabled {
        return Ok(());
    }

    if state.download_state != "ready" || state.available_version.is_none() {
        return Ok(());
    }

    let version = state.available_version.as_deref().unwrap().to_string();

    // Check if we're inside the maintenance window
    let now = chrono::Local::now();
    if !is_in_maintenance_window(
        now.time(),
        &state.auto_update_window_start,
        &state.auto_update_window_end,
    ) {
        // Check if we should send a pre-window notification
        try_send_pre_window_notification(
            now.time(),
            &state.auto_update_window_start,
            state.auto_update_notify_before_minutes,
            &version,
            event_bus,
        );
        return Ok(());
    }

    // Inside the window — check for active deploys
    if db.has_active_deploys().await? {
        info!("auto-update: active deploys detected, waiting {DEPLOY_WAIT_INTERVAL:?}");
        return Ok(());
    }

    let binary_path = match state.download_path.as_deref() {
        Some(p) => p.to_string(),
        None => return Err("no download path".into()),
    };

    info!(
        version,
        "auto-update: applying update during maintenance window"
    );

    let applier = UpdateApplier::new(&config.data_dir);

    event_bus.emit(
        EventType::UpdateStep,
        None,
        None,
        serde_json::json!({
            "step": "auto_apply",
            "status": "running",
            "version": version,
        }),
    );

    let from_version = CURRENT_VERSION.to_string();
    let result = applier
        .apply(
            std::path::Path::new(&binary_path),
            &from_version,
            &version,
            db.as_ref(),
            |step, status| {
                info!("auto-update apply: {step} = {status}");
            },
        )
        .await;

    match result {
        Ok(()) => {
            info!("auto-update: applied successfully");
            let entry = crate::db::models::UpdateHistoryEntry {
                id: crate::db::models::new_id(),
                version: version.clone(),
                previous_version: from_version,
                status: "success".to_string(),
                duration_secs: None,
                error: None,
                changelog_url: None,
                applied_at: now_iso8601(),
            };
            let _ = db.record_update_history(&entry).await;
            let _ = db.clear_update_available().await;
            let _ = db.set_auto_update_pre_downloaded(false).await;
        }
        Err(e) => {
            error!("auto-update: apply failed: {e}");
            let _ = db.set_update_error(&e.to_string()).await;
            let entry = crate::db::models::UpdateHistoryEntry {
                id: crate::db::models::new_id(),
                version: version.clone(),
                previous_version: from_version,
                status: "failed".to_string(),
                duration_secs: None,
                error: Some(e.to_string()),
                changelog_url: None,
                applied_at: now_iso8601(),
            };
            let _ = db.record_update_history(&entry).await;
        }
    }

    Ok(())
}

fn is_in_maintenance_window(now: chrono::NaiveTime, window_start: &str, window_end: &str) -> bool {
    let start = match chrono::NaiveTime::parse_from_str(window_start, "%H:%M") {
        Ok(t) => t,
        Err(_) => return false,
    };
    let end = match chrono::NaiveTime::parse_from_str(window_end, "%H:%M") {
        Ok(t) => t,
        Err(_) => return false,
    };

    if start <= end {
        // Normal window: e.g. 03:00 - 05:00
        now >= start && now < end
    } else {
        // Wrapping window: e.g. 23:00 - 02:00
        now >= start || now < end
    }
}

fn try_send_pre_window_notification(
    now: chrono::NaiveTime,
    window_start: &str,
    notify_before_minutes: i64,
    version: &str,
    event_bus: &Arc<EventBus>,
) {
    let start = match chrono::NaiveTime::parse_from_str(window_start, "%H:%M") {
        Ok(t) => t,
        Err(_) => return,
    };

    let notify_duration = chrono::TimeDelta::minutes(notify_before_minutes);
    let notify_time = start - notify_duration;

    // Check if we're in the notification window (within 1 minute of the notify time)
    let diff = now.signed_duration_since(notify_time);
    if diff >= chrono::TimeDelta::zero() && diff < chrono::TimeDelta::minutes(1) {
        info!("auto-update: sending pre-window notification for v{version}");
        event_bus.emit(
            EventType::UpdateScheduled,
            None,
            None,
            serde_json::json!({
                "version": version,
                "window_start": window_start,
                "minutes_until": notify_before_minutes,
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_window_inside() {
        let now = chrono::NaiveTime::from_hms_opt(3, 30, 0).unwrap();
        assert!(is_in_maintenance_window(now, "03:00", "05:00"));
    }

    #[test]
    fn normal_window_outside_before() {
        let now = chrono::NaiveTime::from_hms_opt(2, 30, 0).unwrap();
        assert!(!is_in_maintenance_window(now, "03:00", "05:00"));
    }

    #[test]
    fn normal_window_outside_after() {
        let now = chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap();
        assert!(!is_in_maintenance_window(now, "03:00", "05:00"));
    }

    #[test]
    fn normal_window_at_start_boundary() {
        let now = chrono::NaiveTime::from_hms_opt(3, 0, 0).unwrap();
        assert!(is_in_maintenance_window(now, "03:00", "05:00"));
    }

    #[test]
    fn normal_window_at_end_boundary() {
        let now = chrono::NaiveTime::from_hms_opt(5, 0, 0).unwrap();
        assert!(!is_in_maintenance_window(now, "03:00", "05:00"));
    }

    #[test]
    fn wrapping_window_late_night() {
        let now = chrono::NaiveTime::from_hms_opt(23, 30, 0).unwrap();
        assert!(is_in_maintenance_window(now, "23:00", "02:00"));
    }

    #[test]
    fn wrapping_window_early_morning() {
        let now = chrono::NaiveTime::from_hms_opt(1, 0, 0).unwrap();
        assert!(is_in_maintenance_window(now, "23:00", "02:00"));
    }

    #[test]
    fn wrapping_window_outside() {
        let now = chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        assert!(!is_in_maintenance_window(now, "23:00", "02:00"));
    }

    #[test]
    fn wrapping_window_at_end_boundary() {
        let now = chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap();
        assert!(!is_in_maintenance_window(now, "23:00", "02:00"));
    }

    #[test]
    fn invalid_window_times() {
        let now = chrono::NaiveTime::from_hms_opt(3, 0, 0).unwrap();
        assert!(!is_in_maintenance_window(now, "invalid", "05:00"));
        assert!(!is_in_maintenance_window(now, "03:00", "invalid"));
    }
}
