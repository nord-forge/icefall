use std::sync::Arc;

use tracing::{error, info};

use crate::config::IcefallConfig;
use crate::db::models::now_iso8601;
use crate::db::Database;
use crate::events::{EventBus, EventType};
use crate::update::apply::UpdateApplier;
use crate::update::CURRENT_VERSION;

use super::DEPLOY_WAIT_INTERVAL;

pub(super) async fn try_auto_apply(
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

    let version = state.available_version.as_deref().expect("guarded by is_none() check").to_string();

    let now = chrono::Local::now();
    if !is_in_maintenance_window(
        now.time(),
        &state.auto_update_window_start,
        &state.auto_update_window_end,
    ) {
        try_send_pre_window_notification(
            now.time(),
            &state.auto_update_window_start,
            state.auto_update_notify_before_minutes,
            &version,
            event_bus,
        );
        return Ok(());
    }

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

pub(super) fn is_in_maintenance_window(
    now: chrono::NaiveTime,
    window_start: &str,
    window_end: &str,
) -> bool {
    let Ok(start) = chrono::NaiveTime::parse_from_str(window_start, "%H:%M") else {
        return false;
    };
    let Ok(end) = chrono::NaiveTime::parse_from_str(window_end, "%H:%M") else {
        return false;
    };

    if start <= end {
        now >= start && now < end
    } else {
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
    let Ok(start) = chrono::NaiveTime::parse_from_str(window_start, "%H:%M") else {
        return;
    };

    let notify_duration = chrono::TimeDelta::minutes(notify_before_minutes);
    let notify_time = start - notify_duration;

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
