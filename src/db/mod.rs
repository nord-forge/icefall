pub mod encryption;
pub mod models;
pub mod sqlite;

use async_trait::async_trait;
use thiserror::Error;

use crate::db::models::*;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("duplicate record: {0}")]
    Duplicate(String),
    #[error("encryption error: {0}")]
    Encryption(#[from] encryption::EncryptionError),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[async_trait]
pub trait Database: Send + Sync + 'static {
    // Projects
    async fn list_projects(&self) -> Result<Vec<Project>, DbError>;
    async fn create_project(&self, project: &NewProject) -> Result<Project, DbError>;
    async fn get_project(&self, id: &str) -> Result<Option<Project>, DbError>;
    async fn update_project(&self, id: &str, update: &UpdateProject) -> Result<Project, DbError>;
    async fn delete_project(&self, id: &str) -> Result<(), DbError>;

    // Apps
    async fn create_app(&self, app: &NewApp) -> Result<App, DbError>;
    async fn get_app(&self, id: &str) -> Result<Option<App>, DbError>;
    async fn get_app_by_name(&self, name: &str) -> Result<Option<App>, DbError>;
    async fn list_apps(&self) -> Result<Vec<App>, DbError>;
    async fn list_apps_by_project(&self, project_id: &str) -> Result<Vec<App>, DbError>;
    async fn update_app(&self, id: &str, update: &UpdateApp) -> Result<App, DbError>;
    async fn delete_app(&self, id: &str) -> Result<(), DbError>;

    // Environments
    async fn create_environment(&self, env: &NewEnvironment) -> Result<Environment, DbError>;
    async fn list_environments(&self, app_id: &str) -> Result<Vec<Environment>, DbError>;
    async fn delete_environment(&self, id: &str) -> Result<(), DbError>;

    // Env Vars
    async fn set_env_var(&self, env_var: &NewEnvVar) -> Result<EnvVar, DbError>;
    async fn get_env_vars(&self, environment_id: &str) -> Result<Vec<EnvVar>, DbError>;
    async fn delete_env_var(&self, id: &str) -> Result<(), DbError>;

    // Deploys
    async fn create_deploy(&self, deploy: &NewDeploy) -> Result<Deploy, DbError>;
    async fn get_deploy(&self, id: &str) -> Result<Option<Deploy>, DbError>;
    async fn list_deploys(&self, app_id: &str, limit: i64) -> Result<Vec<Deploy>, DbError>;
    async fn get_latest_deploys_for_apps(&self, app_ids: &[String])
        -> Result<Vec<Deploy>, DbError>;
    async fn update_deploy_status(
        &self,
        id: &str,
        status: &str,
        log: Option<&str>,
    ) -> Result<(), DbError>;

    // Managed Databases
    async fn create_managed_db(&self, db: &NewManagedDatabase) -> Result<ManagedDatabase, DbError>;
    async fn list_managed_dbs(&self) -> Result<Vec<ManagedDatabase>, DbError>;
    async fn list_managed_dbs_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ManagedDatabase>, DbError>;
    async fn update_managed_db_credentials(
        &self,
        id: &str,
        credentials_json: &str,
        container_id: &str,
    ) -> Result<(), DbError>;
    async fn delete_managed_db(&self, id: &str) -> Result<(), DbError>;

    // Domains
    async fn add_domain(&self, domain: &NewDomain) -> Result<Domain, DbError>;
    async fn list_domains(&self, app_id: &str) -> Result<Vec<Domain>, DbError>;
    async fn update_domain_status(
        &self,
        id: &str,
        verified: bool,
        ssl_status: &str,
    ) -> Result<(), DbError>;
    async fn delete_domain(&self, id: &str) -> Result<(), DbError>;

    // Users
    async fn create_user(&self, user: &NewUser) -> Result<User, DbError>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, DbError>;
    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, DbError>;
    async fn list_users(&self) -> Result<Vec<User>, DbError>;

    // TOTP / 2FA
    async fn update_user_totp_secret(
        &self,
        user_id: &str,
        secret: Option<&str>,
    ) -> Result<(), DbError>;
    async fn enable_user_totp(&self, user_id: &str, backup_codes: &str) -> Result<(), DbError>;
    async fn disable_user_totp(&self, user_id: &str) -> Result<(), DbError>;
    async fn update_user_backup_codes(
        &self,
        user_id: &str,
        backup_codes: &str,
    ) -> Result<(), DbError>;

    // Server Metrics (legacy single-server)
    async fn insert_server_metric(
        &self,
        snapshot: &crate::api::routes::server::ServerMetricsSnapshot,
    ) -> Result<(), DbError>;
    async fn query_server_metrics(
        &self,
        from: &str,
        to: &str,
        limit: usize,
    ) -> Result<Vec<crate::api::routes::server::ServerMetricsSnapshot>, DbError>;
    async fn prune_server_metrics(&self, older_than: &str) -> Result<u64, DbError>;

    // Servers
    async fn create_server(&self, server: &NewServer) -> Result<Server, DbError>;
    async fn get_server(&self, id: &str) -> Result<Option<Server>, DbError>;
    async fn get_server_by_token_hash(&self, hash: &str) -> Result<Option<Server>, DbError>;
    async fn list_servers(&self) -> Result<Vec<Server>, DbError>;
    async fn update_server(&self, id: &str, update: &ServerUpdate) -> Result<Server, DbError>;
    async fn delete_server(&self, id: &str) -> Result<(), DbError>;
    async fn update_server_heartbeat(&self, id: &str) -> Result<(), DbError>;
    async fn update_server_status(&self, id: &str, status: &str) -> Result<(), DbError>;

    // Server Metrics History (multi-server)
    async fn insert_server_metrics_record(
        &self,
        record: &NewServerMetricsRecord,
    ) -> Result<ServerMetricsRecord, DbError>;
    async fn query_server_metrics_history(
        &self,
        server_id: &str,
        from: &str,
        to: &str,
        limit: usize,
    ) -> Result<Vec<ServerMetricsRecord>, DbError>;
    async fn prune_server_metrics_history(&self, older_than: &str) -> Result<u64, DbError>;

    // Health Checks
    async fn create_health_check(&self, hc: &NewHealthCheck) -> Result<HealthCheck, DbError>;
    async fn get_health_checks(&self, app_id: &str) -> Result<Vec<HealthCheck>, DbError>;
    async fn record_health_event(&self, event: &NewHealthCheckEvent) -> Result<(), DbError>;
    async fn get_health_events(
        &self,
        health_check_id: &str,
        limit: i64,
    ) -> Result<Vec<HealthCheckEvent>, DbError>;

    // Notifications
    async fn create_notification_channel(
        &self,
        channel: &NewNotification,
    ) -> Result<Notification, DbError>;
    async fn list_notification_channels(&self) -> Result<Vec<Notification>, DbError>;
    async fn create_notification_rule(
        &self,
        rule: &NewNotificationRule,
    ) -> Result<NotificationRule, DbError>;
    async fn get_notification_rules(&self, app_id: &str) -> Result<Vec<NotificationRule>, DbError>;

    // Lookup helpers
    async fn get_app_by_repo(&self, repo_url: &str) -> Result<Option<App>, DbError>;
    async fn get_environment_by_branch(
        &self,
        app_id: &str,
        branch: &str,
    ) -> Result<Option<Environment>, DbError>;

    // Deploy extras
    async fn update_deploy_container_id(
        &self,
        deploy_id: &str,
        container_id: &str,
    ) -> Result<(), DbError>;
    async fn update_deploy_image_ref(
        &self,
        deploy_id: &str,
        image_ref: &str,
    ) -> Result<(), DbError>;
    async fn update_deploy_env_snapshot(
        &self,
        deploy_id: &str,
        env_snapshot: &str,
    ) -> Result<(), DbError>;

    // User profile updates
    async fn update_user_password(&self, user_id: &str, password_hash: &str)
        -> Result<(), DbError>;
    async fn update_user_email(&self, user_id: &str, email: &str) -> Result<(), DbError>;

    // Sessions
    async fn create_session(&self, user_id: &str, expires_at: &str) -> Result<Session, DbError>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, DbError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), DbError>;
    async fn delete_user_sessions(&self, user_id: &str) -> Result<(), DbError>;
    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, DbError>;
    async fn delete_user_sessions_except(
        &self,
        user_id: &str,
        keep_session_id: &str,
    ) -> Result<(), DbError>;

    // API Tokens
    async fn create_api_token(
        &self,
        user_id: &str,
        name: &str,
        token_hash: &str,
        expires_at: Option<&str>,
    ) -> Result<ApiToken, DbError>;
    async fn get_api_token_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, DbError>;
    async fn list_api_tokens(&self, user_id: &str) -> Result<Vec<ApiToken>, DbError>;
    async fn delete_api_token(&self, id: &str) -> Result<(), DbError>;
    async fn update_token_last_used(&self, id: &str) -> Result<(), DbError>;

    // Invitations
    async fn create_invitation(
        &self,
        email: &str,
        role: &str,
        token: &str,
        expires_at: &str,
    ) -> Result<Invitation, DbError>;
    async fn get_invitation_by_token(&self, token: &str) -> Result<Option<Invitation>, DbError>;
    async fn delete_invitation(&self, id: &str) -> Result<(), DbError>;

    // Env var extras
    async fn delete_env_vars_by_environment(&self, environment_id: &str) -> Result<(), DbError>;

    // Onboarding
    async fn get_onboarding(
        &self,
    ) -> Result<Option<(String, String, String, Option<String>)>, DbError>;
    async fn create_onboarding(&self, started_at: &str) -> Result<(), DbError>;
    async fn update_onboarding_state(
        &self,
        current_step: &str,
        completed_steps: &str,
        completed_at: Option<&str>,
    ) -> Result<(), DbError>;

    // Instance Backup
    async fn get_instance_backup_config(&self) -> Result<Option<InstanceBackupConfig>, DbError>;
    async fn upsert_instance_backup_config(
        &self,
        enabled: bool,
        cron_schedule: &str,
        retention_count: i64,
    ) -> Result<InstanceBackupConfig, DbError>;
    async fn create_instance_backup_record(
        &self,
        filename: &str,
        s3_key: Option<&str>,
    ) -> Result<InstanceBackupRecord, DbError>;
    async fn update_instance_backup_record(
        &self,
        id: &str,
        status: &str,
        size_bytes: i64,
        error_message: Option<&str>,
    ) -> Result<(), DbError>;
    async fn list_instance_backup_history(
        &self,
        limit: i64,
    ) -> Result<Vec<InstanceBackupRecord>, DbError>;
    async fn delete_instance_backup_record(&self, id: &str) -> Result<(), DbError>;

    // OAuth Identities
    async fn create_oauth_identity(
        &self,
        user_id: &str,
        provider: &str,
        provider_user_id: &str,
        provider_email: Option<&str>,
    ) -> Result<OAuthIdentity, DbError>;
    async fn get_oauth_identity(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> Result<Option<OAuthIdentity>, DbError>;
    async fn list_oauth_identities_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<OAuthIdentity>, DbError>;
    async fn delete_oauth_identity(&self, id: &str) -> Result<(), DbError>;

    // OAuth Settings
    async fn get_oauth_settings(&self) -> Result<Option<OAuthSettings>, DbError>;
    async fn upsert_oauth_settings(&self, settings: &OAuthSettings) -> Result<(), DbError>;

    // Registration Settings
    async fn get_registration_settings(&self) -> Result<RegistrationSettings, DbError>;
    async fn upsert_registration_settings(
        &self,
        allow_registration: bool,
        allowed_domains: Option<&str>,
        default_role: &str,
    ) -> Result<RegistrationSettings, DbError>;

    // User deletion
    async fn delete_user(&self, user_id: &str) -> Result<(), DbError>;
    async fn count_admin_users(&self) -> Result<i64, DbError>;

    // User preferences
    async fn get_user_preferences(&self, user_id: &str) -> Result<serde_json::Value, DbError>;
    async fn update_user_preferences(
        &self,
        user_id: &str,
        preferences: &serde_json::Value,
    ) -> Result<(), DbError>;

    // Admin 2FA reset
    async fn admin_reset_user_2fa(&self, user_id: &str) -> Result<(), DbError>;

    // Audit log
    async fn create_audit_log(&self, entry: &NewAuditLogEntry) -> Result<(), DbError>;
    async fn list_audit_logs(
        &self,
        server_id: Option<&str>,
        action: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, DbError>;
    async fn prune_audit_logs(&self, older_than: &str) -> Result<u64, DbError>;

    // Update state
    async fn get_update_state(&self) -> Result<UpdateState, DbError>;
    async fn set_update_available(
        &self,
        version: &str,
        release_url: &str,
        release_notes: &str,
        highlights: &str,
    ) -> Result<(), DbError>;
    async fn set_update_download_state(
        &self,
        state: &str,
        progress: i64,
        path: Option<&str>,
    ) -> Result<(), DbError>;
    async fn set_update_error(&self, error: &str) -> Result<(), DbError>;
    async fn clear_update_available(&self) -> Result<(), DbError>;
    async fn update_highest_seen(&self, version: &str) -> Result<(), DbError>;
    async fn set_last_check_at(&self, timestamp: &str) -> Result<(), DbError>;

    // Update preferences
    async fn set_update_channel(&self, channel: &str) -> Result<(), DbError>;
    async fn set_auto_update_settings(
        &self,
        enabled: bool,
        channel: &str,
        window_start: &str,
        window_end: &str,
        notify_before_minutes: i64,
    ) -> Result<(), DbError>;
    async fn set_auto_update_pre_downloaded(&self, pre_downloaded: bool) -> Result<(), DbError>;
    async fn has_active_deploys(&self) -> Result<bool, DbError>;

    // Skipped versions
    async fn skip_update_version(&self, version: &str) -> Result<(), DbError>;
    async fn is_version_skipped(&self, version: &str) -> Result<bool, DbError>;

    // Update history
    async fn record_update_history(&self, entry: &UpdateHistoryEntry) -> Result<(), DbError>;
    async fn list_update_history(&self, limit: usize) -> Result<Vec<UpdateHistoryEntry>, DbError>;

    // Backup
    async fn vacuum_into(&self, path: &str) -> Result<(), DbError>;

    // Migrations
    async fn run_migrations(&self) -> Result<(), DbError>;
}
