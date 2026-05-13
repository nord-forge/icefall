mod auto_apply;
mod check;
mod pre_download;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use tracing::{debug, warn};

use crate::config::IcefallConfig;
use crate::db::Database;
use crate::events::EventBus;
use crate::update::discovery::UpdateChecker;

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
            if let Err(e) = check::run_check(&checker, &db, &event_bus).await {
                warn!("update check failed: {e}");
            }

            if let Err(e) = pre_download::try_pre_download(&checker, &db, &config).await {
                debug!("pre-download skipped or failed: {e}");
            }

            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

pub fn spawn_auto_update_applier(
    db: Arc<dyn Database>,
    config: Arc<IcefallConfig>,
    event_bus: Arc<EventBus>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        loop {
            if let Err(e) = auto_apply::try_auto_apply(&db, &config, &event_bus).await {
                debug!("auto-apply skipped: {e}");
            }
            tokio::time::sleep(WINDOW_POLL_INTERVAL).await;
        }
    });
}
