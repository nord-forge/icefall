use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InstanceBackupConfig {
    pub id: String,
    pub enabled: bool,
    pub cron_schedule: String,
    pub retention_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InstanceBackupRecord {
    pub id: String,
    pub filename: String,
    pub size_bytes: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub s3_key: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthIdentity {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSettings {
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub github_enabled: bool,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistrationSettings {
    pub id: String,
    pub allow_registration: bool,
    pub allowed_domains: Option<String>,
    pub default_role: String,
    pub updated_at: String,
}
