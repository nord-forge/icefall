use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait UpdateStore: Send + Sync {
    // Update state
    async fn get_update_state(&self) -> Result<UpdateState, DbError>;
    async fn set_update_available(
        &self,
        version: &str,
        release_url: &str,
        release_notes: &str,
        highlights: &str,
    ) -> Result<(), DbError>;
    async fn set_update_download_state(
        &self,
        state: &str,
        progress: i64,
        path: Option<&str>,
    ) -> Result<(), DbError>;
    async fn set_update_error(&self, error: &str) -> Result<(), DbError>;
    async fn clear_update_available(&self) -> Result<(), DbError>;
    async fn update_highest_seen(&self, version: &str) -> Result<(), DbError>;
    async fn set_last_check_at(&self, timestamp: &str) -> Result<(), DbError>;

    // Update preferences
    async fn set_update_channel(&self, channel: &str) -> Result<(), DbError>;
    async fn set_auto_update_settings(
        &self,
        enabled: bool,
        channel: &str,
        window_start: &str,
        window_end: &str,
        notify_before_minutes: i64,
    ) -> Result<(), DbError>;
    async fn set_auto_update_pre_downloaded(&self, pre_downloaded: bool) -> Result<(), DbError>;
    async fn has_active_deploys(&self) -> Result<bool, DbError>;

    // Skipped versions
    async fn skip_update_version(&self, version: &str) -> Result<(), DbError>;
    async fn is_version_skipped(&self, version: &str) -> Result<bool, DbError>;

    // Update history
    async fn record_update_history(&self, entry: &UpdateHistoryEntry) -> Result<(), DbError>;
    async fn list_update_history(&self, limit: usize) -> Result<Vec<UpdateHistoryEntry>, DbError>;
}
