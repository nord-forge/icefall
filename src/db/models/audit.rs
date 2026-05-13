use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLogEntry {
    pub id: String,
    pub server_id: Option<String>,
    pub user_id: Option<String>,
    pub action: String,
    pub details: String,
    pub ip_address: Option<String>,
    pub created_at: String,
}

pub struct NewAuditLogEntry {
    pub server_id: Option<String>,
    pub user_id: Option<String>,
    pub action: String,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
}
