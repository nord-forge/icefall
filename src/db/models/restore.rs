use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DatabaseRestoreRecord {
    pub id: String,
    pub database_id: String,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub status: String,
    pub output: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}
