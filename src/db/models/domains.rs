use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Domain {
    pub id: String,
    pub app_id: String,
    pub domain: String,
    pub path: Option<String>,
    pub verified: bool,
    pub ssl_status: String,
    pub is_primary: bool,
    pub ssl_issuer: Option<String>,
    pub ssl_expires_at: Option<String>,
    pub ssl_last_checked_at: Option<String>,
    pub created_at: String,
}

pub struct NewDomain {
    pub app_id: String,
    pub domain: String,
    pub path: Option<String>,
}
