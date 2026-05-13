use std::path::{Path, PathBuf};

use tracing::{error, info, warn};

use crate::update::apply::{PendingUpdate, DASHBOARD_DIR};
use crate::update::UpdateError;

use super::UpdateRollback;

impl UpdateRollback {
    pub fn execute_rollback(&self) -> Result<(), UpdateError> {
        let marker_path = self.data_dir.join("updates").join("pending_update");
        let content = std::fs::read_to_string(&marker_path)
            .map_err(|e| UpdateError::Rollback(format!("no pending update marker: {e}")))?;
        let marker: PendingUpdate = serde_json::from_str(&content)
            .map_err(|e| UpdateError::Rollback(format!("invalid marker: {e}")))?;

        info!(
            from = marker.to_version,
            to = marker.from_version,
            "executing rollback"
        );

        let rollback_binary = Path::new(&marker.rollback_binary);
        if rollback_binary.exists() {
            std::fs::copy(rollback_binary, &self.binary_path).map_err(|e| {
                UpdateError::Rollback(format!(
                    "failed to restore binary from {}: {e}",
                    rollback_binary.display()
                ))
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&self.binary_path, std::fs::Permissions::from_mode(0o755))
                    .map_err(|e| {
                        UpdateError::Rollback(format!("failed to set binary permissions: {e}"))
                    })?;
            }

            info!(path = %self.binary_path.display(), "binary restored from rollback");
        } else {
            error!(path = %rollback_binary.display(), "rollback binary not found, skipping binary restore");
        }

        let db_backup = Path::new(&marker.db_backup);
        let db_path = self.data_dir.join("icefall.db");
        if db_backup.exists() {
            std::fs::copy(db_backup, &db_path).map_err(|e| {
                UpdateError::Rollback(format!(
                    "failed to restore database from {}: {e}",
                    db_backup.display()
                ))
            })?;
            info!(path = %db_path.display(), "database restored from backup");
        } else {
            warn!(path = %db_backup.display(), "database backup not found, skipping database restore");
        }

        if let Some(ref dashboard_bak) = marker.dashboard_backup {
            let bak_path = Path::new(dashboard_bak);
            let dashboard_path = PathBuf::from(DASHBOARD_DIR);
            if bak_path.exists() {
                if dashboard_path.exists() {
                    let _ = std::fs::remove_dir_all(&dashboard_path);
                }
                if let Err(e) = std::fs::rename(bak_path, &dashboard_path) {
                    warn!(error = %e, "failed to restore dashboard assets, trying copy");
                    if let Err(e2) =
                        crate::update::apply::copy_dir_recursive(bak_path, &dashboard_path)
                    {
                        error!(error = %e2, "failed to copy dashboard assets during rollback");
                    }
                }
                info!("dashboard assets restored from backup");
            } else {
                warn!(path = %bak_path.display(), "dashboard backup not found, skipping dashboard restore");
            }
        }

        if let Err(e) = std::fs::remove_file(&marker_path) {
            warn!(error = %e, "failed to remove pending update marker");
        }

        if std::env::var("INVOCATION_ID").is_ok() {
            info!("triggering restart via systemd after rollback");
            let _ = std::process::Command::new("systemctl")
                .args(["restart", "icefall"])
                .spawn();
        }

        Ok(())
    }
}
