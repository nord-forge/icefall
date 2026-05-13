use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UpdateState {
    pub id: i64,
    pub highest_seen_version: String,
    pub available_version: Option<String>,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub changelog_highlights: Option<String>,
    pub channel: String,
    pub download_state: String,
    pub download_progress: Option<i64>,
    pub download_path: Option<String>,
    pub last_check_at: Option<String>,
    pub last_update_at: Option<String>,
    pub last_update_version: Option<String>,
    pub error_message: Option<String>,
    pub auto_update_enabled: bool,
    pub auto_update_channel: String,
    pub auto_update_window_start: String,
    pub auto_update_window_end: String,
    pub auto_update_notify_before_minutes: i64,
    pub auto_update_pre_downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UpdateHistoryEntry {
    pub id: String,
    pub version: String,
    pub previous_version: String,
    pub status: String,
    pub duration_secs: Option<f64>,
    pub error: Option<String>,
    pub changelog_url: Option<String>,
    pub applied_at: String,
}
