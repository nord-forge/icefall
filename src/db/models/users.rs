use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    #[serde(skip_serializing)]
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
    #[serde(skip_serializing)]
    pub totp_backup_codes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub role: String,
}
