use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CleanupSchedule {
    pub id: String,
    pub enabled: bool,
    pub cron_expression: String,
    pub disk_threshold_percent: i32,
    pub cleanup_dangling_images: bool,
    pub cleanup_unused_images: bool,
    pub cleanup_stopped_containers: bool,
    pub cleanup_unused_volumes: bool,
    pub cleanup_unused_networks: bool,
    pub stopped_container_age_hours: i32,
    pub updated_at: String,
}
