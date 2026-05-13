use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HealthCheck {
    pub id: String,
    pub app_id: String,
    pub check_type: String,
    pub config: Option<String>,
    pub interval_secs: i64,
    pub failure_threshold: i64,
    pub auto_restart: bool,
    pub created_at: String,
}

pub struct NewHealthCheck {
    pub app_id: String,
    pub check_type: String,
    pub config: Option<String>,
    pub interval_secs: i64,
    pub failure_threshold: i64,
    pub auto_restart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HealthCheckEvent {
    pub id: String,
    pub health_check_id: String,
    pub status: String,
    pub checked_at: String,
}

pub struct NewHealthCheckEvent {
    pub health_check_id: String,
    pub status: String,
}
