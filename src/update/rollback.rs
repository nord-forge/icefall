use std::path::{Path, PathBuf};

use tracing::{error, info, warn};

use crate::update::apply::{PendingUpdate, DASHBOARD_DIR};
use crate::update::UpdateError;

/// Handles automatic and manual rollback of failed updates.
///
/// On each daemon startup, if a pending update marker is found and the process
/// has not passed its health checks within 5 minutes, the rollback module
/// restores the previous binary and database and restarts the daemon.
pub struct UpdateRollback {
    data_dir: PathBuf,
    binary_path: PathBuf,
}

/// Metadata about the available rollback binary.
pub struct RollbackInfo {
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

impl UpdateRollback {
    pub fn new(data_dir: &Path) -> Self {
        let binary_path =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/local/bin/icefall"));
        Self {
            data_dir: data_dir.to_path_buf(),
            binary_path,
        }
    }

    /// Check whether a rollback is needed.
    ///
    /// Returns `true` when a pending update marker exists and was written less
    /// than 5 minutes ago.  This is called from `ExecStopPost=` in the systemd
    /// unit or from daemon startup to detect a crash-looping update.
    pub fn needs_rollback(&self) -> bool {
        let marker_path = self.data_dir.join("updates").join("pending_update");
        let content = match std::fs::read_to_string(&marker_path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let marker: PendingUpdate = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "corrupt pending update marker, treating as no rollback needed");
                return false;
            }
        };

        match chrono::DateTime::parse_from_rfc3339(&marker.started_at) {
            Ok(started) => {
                let age = chrono::Utc::now().signed_duration_since(started);
                let needs = age.num_seconds() < 300;
                if needs {
                    info!(
                        age_secs = age.num_seconds(),
                        from = marker.from_version,
                        to = marker.to_version,
                        "pending update detected, rollback may be required"
                    );
                }
                needs
            }
            Err(e) => {
                warn!(error = %e, "invalid timestamp in pending update marker");
                false
            }
        }
    }

    /// Execute rollback: restore the previous binary and database, then restart.
    ///
    /// Reads the pending update marker to find the paths to the backed-up
    /// binary and database, copies them back, removes the marker, and triggers
    /// a systemd restart if applicable.
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

        // Restore binary
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

        // Restore database
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

        // Restore dashboard assets
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

        // Remove marker so we don't loop
        if let Err(e) = std::fs::remove_file(&marker_path) {
            warn!(error = %e, "failed to remove pending update marker");
        }

        // Restart if under systemd
        if std::env::var("INVOCATION_ID").is_ok() {
            info!("triggering restart via systemd after rollback");
            let _ = std::process::Command::new("systemctl")
                .args(["restart", "icefall"])
                .spawn();
        }

        Ok(())
    }

    /// Check whether a rollback binary is available on disk.
    pub fn has_rollback(&self) -> bool {
        self.data_dir
            .join("updates")
            .join("icefall.rollback")
            .exists()
    }

    /// Return metadata about the rollback binary, if one exists.
    pub fn rollback_info(&self) -> Option<RollbackInfo> {
        let path = self.data_dir.join("updates").join("icefall.rollback");
        let metadata = std::fs::metadata(&path).ok()?;

        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

        Some(RollbackInfo {
            path: path.to_string_lossy().to_string(),
            size_bytes: metadata.len(),
            modified_at,
        })
    }

    /// Entry point for `ExecStopPost` in the systemd unit.
    ///
    /// Checks if a pending update marker exists and is recent (< 5 min).
    /// If so, executes a full rollback and exits. Otherwise, exits silently.
    /// Returns the process exit code (0 = no action, 1 = rollback executed, 2 = error).
    pub fn check_and_rollback(&self) -> i32 {
        if !self.needs_rollback() {
            return 0;
        }

        info!("rollback check: pending update is recent, executing rollback");
        match self.execute_rollback() {
            Ok(()) => {
                info!("rollback completed successfully");
                1
            }
            Err(e) => {
                error!(error = %e, "rollback failed");
                2
            }
        }
    }

    /// Remove old rollback files that exceed `max_age_days` in age.
    ///
    /// Keeps disk usage in check after a successful update has been running
    /// long enough that the old binary is no longer useful.
    pub fn cleanup_old_rollbacks(&self, max_age_days: u64) -> Result<(), UpdateError> {
        let rollback_path = self.data_dir.join("updates").join("icefall.rollback");
        if let Ok(metadata) = std::fs::metadata(&rollback_path) {
            if let Ok(modified) = metadata.modified() {
                let age = std::time::SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default();

                if age.as_secs() >= max_age_days * 86400 {
                    std::fs::remove_file(&rollback_path)?;
                    info!(days = max_age_days, "cleaned up old rollback binary");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_rollback(dir: &Path) -> UpdateRollback {
        UpdateRollback {
            data_dir: dir.to_path_buf(),
            binary_path: dir.join("fake-icefall"),
        }
    }

    #[test]
    fn needs_rollback_returns_false_without_marker() {
        let tmp = TempDir::new().unwrap();
        let rb = make_rollback(tmp.path());
        assert!(!rb.needs_rollback());
    }

    #[test]
    fn needs_rollback_returns_true_for_recent_marker() {
        let tmp = TempDir::new().unwrap();
        let marker_dir = tmp.path().join("updates");
        std::fs::create_dir_all(&marker_dir).unwrap();

        let marker = PendingUpdate {
            from_version: "0.1.0".into(),
            to_version: "0.2.0".into(),
            rollback_binary: "/tmp/rollback".into(),
            db_backup: "/tmp/backup.db".into(),
            dashboard_backup: None,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&marker).unwrap();
        std::fs::write(marker_dir.join("pending_update"), json).unwrap();

        let rb = make_rollback(tmp.path());
        assert!(rb.needs_rollback());
    }

    #[test]
    fn needs_rollback_returns_false_for_old_marker() {
        let tmp = TempDir::new().unwrap();
        let marker_dir = tmp.path().join("updates");
        std::fs::create_dir_all(&marker_dir).unwrap();

        let old_time = chrono::Utc::now() - chrono::Duration::minutes(10);
        let marker = PendingUpdate {
            from_version: "0.1.0".into(),
            to_version: "0.2.0".into(),
            rollback_binary: "/tmp/rollback".into(),
            db_backup: "/tmp/backup.db".into(),
            dashboard_backup: None,
            started_at: old_time.to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&marker).unwrap();
        std::fs::write(marker_dir.join("pending_update"), json).unwrap();

        let rb = make_rollback(tmp.path());
        assert!(!rb.needs_rollback());
    }

    #[test]
    fn has_rollback_detects_file() {
        let tmp = TempDir::new().unwrap();
        let rb = make_rollback(tmp.path());
        assert!(!rb.has_rollback());

        let rollback_dir = tmp.path().join("updates");
        std::fs::create_dir_all(&rollback_dir).unwrap();
        std::fs::write(rollback_dir.join("icefall.rollback"), b"binary").unwrap();
        assert!(rb.has_rollback());
    }

    #[test]
    fn rollback_info_returns_metadata() {
        let tmp = TempDir::new().unwrap();
        let rollback_dir = tmp.path().join("updates");
        std::fs::create_dir_all(&rollback_dir).unwrap();
        std::fs::write(rollback_dir.join("icefall.rollback"), b"binary-data").unwrap();

        let rb = make_rollback(tmp.path());
        let info = rb.rollback_info().unwrap();
        assert_eq!(info.size_bytes, 11);
        assert!(info.modified_at.is_some());
    }

    #[test]
    fn rollback_info_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let rb = make_rollback(tmp.path());
        assert!(rb.rollback_info().is_none());
    }

    #[test]
    fn execute_rollback_restores_binary_and_db() {
        let tmp = TempDir::new().unwrap();

        // Set up the current "bad" binary
        let binary_path = tmp.path().join("fake-icefall");
        std::fs::write(&binary_path, b"bad-binary").unwrap();

        // Set up the rollback binary
        let rollback_dir = tmp.path().join("rollback-store");
        std::fs::create_dir_all(&rollback_dir).unwrap();
        let rollback_binary = rollback_dir.join("icefall.rollback");
        std::fs::write(&rollback_binary, b"good-binary").unwrap();

        // Set up the db backup
        let backup_dir = tmp.path().join("backup-store");
        std::fs::create_dir_all(&backup_dir).unwrap();
        let db_backup = backup_dir.join("backup.db");
        std::fs::write(&db_backup, b"good-db-data").unwrap();

        // Write current (bad) database
        let db_path = tmp.path().join("icefall.db");
        std::fs::write(&db_path, b"bad-db-data").unwrap();

        // Write the pending marker
        let marker_dir = tmp.path().join("updates");
        std::fs::create_dir_all(&marker_dir).unwrap();
        let marker = PendingUpdate {
            from_version: "0.1.0".into(),
            to_version: "0.2.0".into(),
            rollback_binary: rollback_binary.to_string_lossy().to_string(),
            db_backup: db_backup.to_string_lossy().to_string(),
            dashboard_backup: None,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&marker).unwrap();
        std::fs::write(marker_dir.join("pending_update"), &json).unwrap();

        let rb = UpdateRollback {
            data_dir: tmp.path().to_path_buf(),
            binary_path: binary_path.clone(),
        };

        rb.execute_rollback().unwrap();

        // Binary should be restored
        assert_eq!(std::fs::read(&binary_path).unwrap(), b"good-binary");
        // Database should be restored
        assert_eq!(std::fs::read(&db_path).unwrap(), b"good-db-data");
        // Marker should be removed
        assert!(!marker_dir.join("pending_update").exists());
    }

    #[test]
    fn execute_rollback_fails_without_marker() {
        let tmp = TempDir::new().unwrap();
        let rb = make_rollback(tmp.path());
        assert!(rb.execute_rollback().is_err());
    }

    #[test]
    fn cleanup_old_rollbacks_removes_expired() {
        let tmp = TempDir::new().unwrap();
        let rollback_dir = tmp.path().join("updates");
        std::fs::create_dir_all(&rollback_dir).unwrap();
        let rollback_path = rollback_dir.join("icefall.rollback");
        std::fs::write(&rollback_path, b"old-binary").unwrap();

        let rb = make_rollback(tmp.path());

        // With max_age_days = 0, anything is expired
        rb.cleanup_old_rollbacks(0).unwrap();
        assert!(!rollback_path.exists());
    }

    #[test]
    fn cleanup_old_rollbacks_keeps_recent() {
        let tmp = TempDir::new().unwrap();
        let rollback_dir = tmp.path().join("updates");
        std::fs::create_dir_all(&rollback_dir).unwrap();
        let rollback_path = rollback_dir.join("icefall.rollback");
        std::fs::write(&rollback_path, b"recent-binary").unwrap();

        let rb = make_rollback(tmp.path());

        // With max_age_days = 30, a just-created file should be kept
        rb.cleanup_old_rollbacks(30).unwrap();
        assert!(rollback_path.exists());
    }
}
