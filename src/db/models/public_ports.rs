use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PublicPort {
    pub id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub port: i32,
    pub ip_whitelist: Option<String>,
    pub created_at: String,
}
