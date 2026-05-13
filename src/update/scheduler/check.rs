use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::db::models::now_iso8601;
use crate::db::Database;
use crate::events::{EventBus, EventType};
use crate::update::discovery::UpdateChecker;
use crate::update::CURRENT_VERSION;

pub(super) async fn run_check(
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
