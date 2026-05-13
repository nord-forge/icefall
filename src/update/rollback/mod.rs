mod execute;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::update::apply::PendingUpdate;
use crate::update::UpdateError;

pub struct UpdateRollback {
    pub(super) data_dir: PathBuf,
    pub(super) binary_path: PathBuf,
}

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

    pub fn has_rollback(&self) -> bool {
        self.data_dir
            .join("updates")
            .join("icefall.rollback")
            .exists()
    }

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
                tracing::error!(error = %e, "rollback failed");
                2
            }
        }
    }

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
