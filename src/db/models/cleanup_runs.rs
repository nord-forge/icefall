use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CleanupRun {
    pub id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub freed_bytes: i64,
    pub removed_items: i64,
    pub error: Option<String>,
    pub details: Option<String>,
}
