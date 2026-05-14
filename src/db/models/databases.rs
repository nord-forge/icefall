use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedDatabase {
    pub id: String,
    pub name: String,
    pub db_type: String,
    pub container_id: Option<String>,
    pub credentials: String,
    pub backup_schedule: Option<String>,
    pub app_id: Option<String>,
    pub project_id: Option<String>,
    pub backup_retention_count: i32,
    pub created_at: String,
}

pub struct NewManagedDatabase {
    pub name: String,
    pub db_type: String,
    pub app_id: Option<String>,
}
