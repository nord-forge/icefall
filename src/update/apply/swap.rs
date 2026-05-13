use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::db::Database;
use crate::update::UpdateError;

use super::{copy_dir_recursive, UpdateApplier, DASHBOARD_DIR};

impl UpdateApplier {
    pub(super) fn backup_binary(&self) -> Result<PathBuf, UpdateError> {
        let rollback_dir = self.data_dir.join("updates");
        std::fs::create_dir_all(&rollback_dir)?;

        let rollback_path = rollback_dir.join("icefall.rollback");
        std::fs::copy(&self.binary_path, &rollback_path).map_err(|e| {
            UpdateError::Apply(format!(
                "failed to backup binary from {} to {}: {e}",
                self.binary_path.display(),
                rollback_path.display()
            ))
        })?;

        Ok(rollback_path)
    }

    pub(super) async fn backup_database(
        &self,
        db: &dyn Database,
    ) -> Result<PathBuf, UpdateError> {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let backup_dir = self.data_dir.join("backups");
        std::fs::create_dir_all(&backup_dir)?;

        let backup_path = backup_dir.join(format!("icefall-{timestamp}-pre-update.db"));
        let backup_path_str = backup_path.to_string_lossy().to_string();

        db.vacuum_into(&backup_path_str).await.map_err(|e| {
            UpdateError::Apply(format!("database backup via VACUUM INTO failed: {e}"))
        })?;

        info!(path = %backup_path.display(), "database backup created via VACUUM INTO");
        Ok(backup_path)
    }

    pub(super) fn backup_dashboard(&self) -> Result<Option<PathBuf>, UpdateError> {
        let dashboard_src = PathBuf::from(DASHBOARD_DIR);
        if !dashboard_src.exists() {
            warn!(
                "dashboard directory not found at {}, skipping backup",
                dashboard_src.display()
            );
            return Ok(None);
        }

        let backup_path = PathBuf::from(format!("{}.bak", DASHBOARD_DIR));

        if backup_path.exists() {
            std::fs::remove_dir_all(&backup_path).map_err(|e| {
                UpdateError::Apply(format!(
                    "failed to remove old dashboard backup at {}: {e}",
                    backup_path.display()
                ))
            })?;
        }

        copy_dir_recursive(&dashboard_src, &backup_path).map_err(|e| {
            UpdateError::Apply(format!(
                "failed to backup dashboard from {} to {}: {e}",
                dashboard_src.display(),
                backup_path.display()
            ))
        })?;

        Ok(Some(backup_path))
    }

    pub(super) async fn run_update_migrations(
        &self,
        new_binary: &Path,
    ) -> Result<(), UpdateError> {
        let output = tokio::process::Command::new(new_binary)
            .args(["db", "migrate"])
            .output()
            .await
            .map_err(|e| {
                UpdateError::Apply(format!(
                    "failed to run migrations from {}: {e}",
                    new_binary.display()
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(UpdateError::Apply(format!(
                "migration failed (exit {}): {}",
                output.status,
                stderr.trim()
            )));
        }

        info!("database migrations from new binary completed successfully");
        Ok(())
    }

    pub(super) fn swap_binary(&self, new_binary: &Path) -> Result<(), UpdateError> {
        let staging = self.binary_path.with_extension("new");

        std::fs::copy(new_binary, &staging).map_err(|e| {
            UpdateError::Apply(format!(
                "failed to stage new binary at {}: {e}",
                staging.display()
            ))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))?;
        }

        std::fs::rename(&staging, &self.binary_path).map_err(|e| {
            let _ = std::fs::remove_file(&staging);
            UpdateError::Apply(format!(
                "failed to rename {} -> {}: {e}",
                staging.display(),
                self.binary_path.display()
            ))
        })?;

        Ok(())
    }

    pub(super) fn trigger_restart(&self) -> Result<(), UpdateError> {
        if Self::is_systemd_managed() {
            info!("triggering restart via systemd");
            std::process::Command::new("systemctl")
                .args(["restart", "icefall"])
                .spawn()
                .map_err(|e| UpdateError::Apply(format!("failed to restart via systemd: {e}")))?;
        } else {
            info!("not running under systemd, skipping automatic restart");
        }

        Ok(())
    }

    pub(super) fn is_systemd_managed() -> bool {
        std::env::var("INVOCATION_ID").is_ok()
    }
}
