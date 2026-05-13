pub mod health;
mod swap;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::db::Database;
use crate::update::UpdateError;

pub(crate) const DASHBOARD_DIR: &str = "dashboard/dist";

pub use health::post_update_check;

pub struct UpdateApplier {
    pub(super) data_dir: PathBuf,
    pub(super) binary_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdate {
    pub from_version: String,
    pub to_version: String,
    pub rollback_binary: String,
    pub db_backup: String,
    #[serde(default)]
    pub dashboard_backup: Option<String>,
    pub started_at: String,
}

impl UpdateApplier {
    pub fn new(data_dir: &Path) -> Self {
        let binary_path =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/local/bin/icefall"));
        Self {
            data_dir: data_dir.to_path_buf(),
            binary_path,
        }
    }

    pub async fn apply(
        &self,
        new_binary_path: &Path,
        from_version: &str,
        to_version: &str,
        db: &dyn Database,
        on_step: impl Fn(&str, &str),
    ) -> Result<(), UpdateError> {
        info!(
            from = from_version,
            to = to_version,
            "starting update apply sequence"
        );

        on_step("backup", "running");
        let rollback_path = self.backup_binary()?;
        on_step("backup", "done");
        info!(path = %rollback_path.display(), "binary backup complete");

        on_step("backup_db", "running");
        let db_backup_path = self.backup_database(db).await?;
        on_step("backup_db", "done");
        info!(path = %db_backup_path.display(), "database backup complete");

        on_step("backup_dashboard", "running");
        let dashboard_backup = self.backup_dashboard()?;
        on_step("backup_dashboard", "done");
        if let Some(ref p) = dashboard_backup {
            info!(path = %p.display(), "dashboard backup complete");
        }

        let marker = PendingUpdate {
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            rollback_binary: rollback_path.to_string_lossy().to_string(),
            db_backup: db_backup_path.to_string_lossy().to_string(),
            dashboard_backup: dashboard_backup.map(|p| p.to_string_lossy().to_string()),
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        self.write_pending_marker(&marker)?;
        info!("pending update marker written");

        on_step("migrate", "running");
        self.run_update_migrations(new_binary_path).await?;
        on_step("migrate", "done");

        on_step("swap", "running");
        self.swap_binary(new_binary_path)?;
        on_step("swap", "done");
        info!("binary swap complete");

        on_step("restart", "running");
        self.trigger_restart()?;

        Ok(())
    }

    pub(super) fn write_pending_marker(&self, marker: &PendingUpdate) -> Result<(), UpdateError> {
        let marker_dir = self.data_dir.join("updates");
        std::fs::create_dir_all(&marker_dir)?;

        let marker_path = marker_dir.join("pending_update");
        let json = serde_json::to_string_pretty(marker)?;
        std::fs::write(&marker_path, json).map_err(|e| {
            UpdateError::Apply(format!(
                "failed to write pending marker at {}: {e}",
                marker_path.display()
            ))
        })?;

        Ok(())
    }

    pub fn read_pending_marker(&self) -> Option<PendingUpdate> {
        let path = self.data_dir.join("updates").join("pending_update");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn clear_pending_marker(&self) -> Result<(), UpdateError> {
        let path = self.data_dir.join("updates").join("pending_update");
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}
