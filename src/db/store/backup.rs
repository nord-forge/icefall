use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait BackupStore: Send + Sync {
    // Instance Backup
    async fn get_instance_backup_config(&self) -> Result<Option<InstanceBackupConfig>, DbError>;
    async fn upsert_instance_backup_config(
        &self,
        enabled: bool,
        cron_schedule: &str,
        retention_count: i64,
    ) -> Result<InstanceBackupConfig, DbError>;
    async fn create_instance_backup_record(
        &self,
        filename: &str,
        s3_key: Option<&str>,
    ) -> Result<InstanceBackupRecord, DbError>;
    async fn update_instance_backup_record(
        &self,
        id: &str,
        status: &str,
        size_bytes: i64,
        error_message: Option<&str>,
    ) -> Result<(), DbError>;
    async fn list_instance_backup_history(
        &self,
        limit: i64,
    ) -> Result<Vec<InstanceBackupRecord>, DbError>;
    /// Mark orphaned "running" backups as failed (startup reconciliation).
    /// Returns how many records were reconciled.
    async fn fail_stale_instance_backups(&self) -> Result<u64, DbError>;
    /// True if any instance backup is currently running. Used to block updates
    /// (which restart the daemon) so a backup is never interrupted mid-write.
    async fn has_running_instance_backup(&self) -> Result<bool, DbError>;
    async fn delete_instance_backup_record(&self, id: &str) -> Result<(), DbError>;

    // Backup
    async fn vacuum_into(&self, path: &str) -> Result<(), DbError>;
}
