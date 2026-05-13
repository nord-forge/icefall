use chrono::Utc;
use serde::{Deserialize, Serialize};

// --- Users ---

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

// --- Apps ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct App {
    pub id: String,
    pub name: String,
    pub git_repo: Option<String>,
    pub git_branch: String,
    pub framework: Option<String>,
    pub build_config: Option<String>,
    pub resource_limits: Option<String>,
    pub preview_enabled: bool,
    pub preview_branch_pattern: Option<String>,
    pub webhook_secret: Option<String>,
    pub tags: Option<String>,
    pub volumes: Option<String>,
    pub image_ref: Option<String>,
    pub compose_content: Option<String>,
    pub project_id: Option<String>,
    pub deploy_mode: String,
    pub server_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewApp {
    pub name: String,
    pub git_repo: Option<String>,
    pub git_branch: String,
    pub framework: Option<String>,
    pub image_ref: Option<String>,
    pub compose_content: Option<String>,
    pub deploy_mode: Option<String>,
    pub server_id: Option<String>,
}

#[derive(Default)]
pub struct UpdateApp {
    pub name: Option<String>,
    pub git_repo: Option<String>,
    pub git_branch: Option<String>,
    pub framework: Option<String>,
    pub build_config: Option<String>,
    pub resource_limits: Option<String>,
    pub preview_enabled: Option<bool>,
    pub preview_branch_pattern: Option<Option<String>>,
    pub tags: Option<String>,
    pub volumes: Option<String>,
    pub image_ref: Option<Option<String>>,
    pub compose_content: Option<Option<String>>,
    pub project_id: Option<Option<String>>,
    pub deploy_mode: Option<String>,
    pub server_id: Option<Option<String>>,
}

// --- Projects ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewProject {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

pub struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub color: Option<Option<String>>,
}

// --- Environments ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Environment {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub env_type: String,
    pub branch: Option<String>,
    pub created_at: String,
}

pub struct NewEnvironment {
    pub app_id: String,
    pub name: String,
    pub env_type: String,
    pub branch: Option<String>,
}

// --- Env Vars ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub id: String,
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub scope: String,
    pub created_at: String,
}

pub struct NewEnvVar {
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub scope: String,
}

// --- Deploys ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Deploy {
    pub id: String,
    pub app_id: String,
    pub environment_id: String,
    pub status: String,
    pub git_sha: Option<String>,
    pub build_log: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub image_ref: Option<String>,
    pub container_id: Option<String>,
    pub env_snapshot: Option<String>,
    pub server_id: Option<String>,
    pub created_at: String,
}

pub struct NewDeploy {
    pub app_id: String,
    pub environment_id: String,
    pub git_sha: Option<String>,
    pub server_id: Option<String>,
}

// --- Managed Databases ---

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
    pub created_at: String,
}

pub struct NewManagedDatabase {
    pub name: String,
    pub db_type: String,
    pub app_id: Option<String>,
}

// --- Domains ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Domain {
    pub id: String,
    pub app_id: String,
    pub domain: String,
    pub path: Option<String>,
    pub verified: bool,
    pub ssl_status: String,
    pub created_at: String,
}

pub struct NewDomain {
    pub app_id: String,
    pub domain: String,
    pub path: Option<String>,
}

// --- Notifications ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub channel_type: String,
    pub config: String,
    pub created_at: String,
}

pub struct NewNotification {
    pub channel_type: String,
    pub config: String,
}

// --- Notification Rules ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationRule {
    pub id: String,
    pub app_id: String,
    pub notification_id: String,
    pub event_type: String,
}

pub struct NewNotificationRule {
    pub app_id: String,
    pub notification_id: String,
    pub event_type: String,
}

// --- Health Checks ---

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

// --- Health Check Events ---

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

// --- Sessions ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub expires_at: String,
    pub created_at: String,
}

// --- API Tokens ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiToken {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub token_hash: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

// --- Invitations ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invitation {
    pub id: String,
    pub email: String,
    pub role: String,
    pub token: String,
    pub expires_at: String,
    pub created_at: String,
}

// --- Instance Backup Config ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InstanceBackupConfig {
    pub id: String,
    pub enabled: bool,
    pub cron_schedule: String,
    pub retention_count: i64,
    pub updated_at: String,
}

// --- Instance Backup History ---

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

// --- OAuth Identities ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthIdentity {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub created_at: String,
}

// --- OAuth Settings ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSettings {
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub github_enabled: bool,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_enabled: bool,
}

// --- Registration Settings ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistrationSettings {
    pub id: String,
    pub allow_registration: bool,
    pub allowed_domains: Option<String>,
    pub default_role: String,
    pub updated_at: String,
}

// --- Update State ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UpdateState {
    pub id: i64,
    pub highest_seen_version: String,
    pub available_version: Option<String>,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub changelog_highlights: Option<String>,
    pub channel: String,
    pub download_state: String,
    pub download_progress: Option<i64>,
    pub download_path: Option<String>,
    pub last_check_at: Option<String>,
    pub last_update_at: Option<String>,
    pub last_update_version: Option<String>,
    pub error_message: Option<String>,
    pub auto_update_enabled: bool,
    pub auto_update_channel: String,
    pub auto_update_window_start: String,
    pub auto_update_window_end: String,
    pub auto_update_notify_before_minutes: i64,
    pub auto_update_pre_downloaded: bool,
}

// --- Update History ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UpdateHistoryEntry {
    pub id: String,
    pub version: String,
    pub previous_version: String,
    pub status: String,
    pub duration_secs: Option<f64>,
    pub error: Option<String>,
    pub changelog_url: Option<String>,
    pub applied_at: String,
}

// --- Servers ---

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

// --- Server Metrics History ---

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

// --- Audit Log ---

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

pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..20)
        .map(|_| chars[rng.random_range(0..chars.len())] as char)
        .collect()
}
