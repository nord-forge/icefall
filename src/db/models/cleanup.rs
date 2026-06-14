use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContainerCleanupExecution {
    pub id: String,
    pub server_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub space_reclaimed_bytes: Option<i64>,
    pub images_removed: i32,
    pub volumes_removed: i32,
    pub networks_removed: i32,
    pub status: String,
}
