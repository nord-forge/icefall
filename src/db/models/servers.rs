use serde::{Deserialize, Serialize};

pub const CONTROL_PLANE_SERVER_ID: &str = "cp_ctrl_0000000001";

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub role: String,
    pub status: String,
    pub token_hash: Option<String>,
    pub agent_version: Option<String>,
    pub labels: Option<String>,
    pub resources: Option<String>,
    pub public_key: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub registered_at: Option<String>,
    pub disk_alert_enabled: bool,
    pub disk_alert_warning_threshold: i32,
    pub disk_alert_critical_threshold: i32,
    pub disk_alert_state: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewServer {
    pub name: String,
    pub host: String,
    pub role: String,
    pub token_hash: Option<String>,
    pub labels: Option<String>,
    pub resources: Option<String>,
    pub public_key: Option<String>,
}

pub struct ServerUpdate {
    pub name: Option<String>,
    pub host: Option<String>,
    pub status: Option<String>,
    pub token_hash: Option<Option<String>>,
    pub agent_version: Option<Option<String>>,
    pub labels: Option<Option<String>>,
    pub resources: Option<Option<String>>,
    pub public_key: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerMetricsRecord {
    pub id: String,
    pub server_id: String,
    pub cpu_percent: Option<f64>,
    pub ram_used_bytes: Option<i64>,
    pub ram_total_bytes: Option<i64>,
    pub disk_used_bytes: Option<i64>,
    pub disk_total_bytes: Option<i64>,
    pub load_average: Option<String>,
    pub recorded_at: String,
}

pub struct NewServerMetricsRecord {
    pub server_id: String,
    pub cpu_percent: Option<f64>,
    pub ram_used_bytes: Option<i64>,
    pub ram_total_bytes: Option<i64>,
    pub disk_used_bytes: Option<i64>,
    pub disk_total_bytes: Option<i64>,
    pub load_average: Option<String>,
}
