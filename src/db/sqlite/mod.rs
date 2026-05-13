mod apps;
mod audit;
mod backups;
mod databases;
mod deploys;
mod domains;
mod environments;
mod health;
mod maintenance;
mod notifications;
mod oauth;
mod onboarding;
mod projects;
mod servers;
mod sessions;
mod updates;
mod users;

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::{Database, DbError};

fn normalize_repo_url(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .replace("https://", "")
        .replace("http://", "")
        .replace("git@", "")
        .replace(':', "/")
        .to_lowercase()
}

pub struct SqliteDatabase {
    pool: SqlitePool,
    encryptor: Arc<Encryptor>,
}

impl SqliteDatabase {
    pub async fn connect(database_url: &str, encryptor: Arc<Encryptor>) -> Result<Self, DbError> {
        let options: SqliteConnectOptions = database_url
            .parse::<SqliteConnectOptions>()
            .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        sqlx::query("PRAGMA journal_size_limit = 67108864")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA auto_vacuum = 2").execute(&pool).await?;
        sqlx::query("PRAGMA wal_autocheckpoint = 5000")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA cache_size = -64000")
            .execute(&pool)
            .await?;

        Ok(Self { pool, encryptor })
    }

    #[cfg(test)]
    pub fn new_with_pool(pool: SqlitePool, encryptor: Arc<Encryptor>) -> Self {
        Self { pool, encryptor }
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    // --- Projects ---

    async fn list_projects(&self) -> Result<Vec<Project>, DbError> {
        projects::list_projects(&self.pool).await
    }

    async fn create_project(&self, project: &NewProject) -> Result<Project, DbError> {
        projects::create_project(&self.pool, project).await
    }

    async fn get_project(&self, id: &str) -> Result<Option<Project>, DbError> {
        projects::get_project(&self.pool, id).await
    }

    async fn update_project(&self, id: &str, update: &UpdateProject) -> Result<Project, DbError> {
        projects::update_project(&self.pool, id, update).await
    }

    async fn delete_project(&self, id: &str) -> Result<(), DbError> {
        projects::delete_project(&self.pool, id).await
    }

    // --- Apps ---

    async fn create_app(&self, app: &NewApp) -> Result<App, DbError> {
        apps::create_app(&self.pool, app).await
    }

    async fn get_app(&self, id: &str) -> Result<Option<App>, DbError> {
        apps::get_app(&self.pool, id).await
    }

    async fn get_app_by_name(&self, name: &str) -> Result<Option<App>, DbError> {
        apps::get_app_by_name(&self.pool, name).await
    }

    async fn list_apps(&self) -> Result<Vec<App>, DbError> {
        apps::list_apps(&self.pool).await
    }

    async fn list_apps_by_project(&self, project_id: &str) -> Result<Vec<App>, DbError> {
        apps::list_apps_by_project(&self.pool, project_id).await
    }

    async fn update_app(&self, id: &str, update: &UpdateApp) -> Result<App, DbError> {
        apps::update_app(&self.pool, id, update).await
    }

    async fn delete_app(&self, id: &str) -> Result<(), DbError> {
        apps::delete_app(&self.pool, id).await
    }

    // --- Environments ---

    async fn create_environment(&self, env: &NewEnvironment) -> Result<Environment, DbError> {
        environments::create_environment(&self.pool, env).await
    }

    async fn list_environments(&self, app_id: &str) -> Result<Vec<Environment>, DbError> {
        environments::list_environments(&self.pool, app_id).await
    }

    async fn delete_environment(&self, id: &str) -> Result<(), DbError> {
        environments::delete_environment(&self.pool, id).await
    }

    // --- Env Vars (encrypted) ---

    async fn set_env_var(&self, env_var: &NewEnvVar) -> Result<EnvVar, DbError> {
        environments::set_env_var(&self.pool, &self.encryptor, env_var).await
    }

    async fn get_env_vars(&self, environment_id: &str) -> Result<Vec<EnvVar>, DbError> {
        environments::get_env_vars(&self.pool, &self.encryptor, environment_id).await
    }

    async fn delete_env_var(&self, id: &str) -> Result<(), DbError> {
        environments::delete_env_var(&self.pool, id).await
    }

    // --- Deploys ---

    async fn create_deploy(&self, deploy: &NewDeploy) -> Result<Deploy, DbError> {
        deploys::create_deploy(&self.pool, deploy).await
    }

    async fn get_deploy(&self, id: &str) -> Result<Option<Deploy>, DbError> {
        deploys::get_deploy(&self.pool, id).await
    }

    async fn list_deploys(&self, app_id: &str, limit: i64) -> Result<Vec<Deploy>, DbError> {
        deploys::list_deploys(&self.pool, app_id, limit).await
    }

    async fn get_latest_deploys_for_apps(
        &self,
        app_ids: &[String],
    ) -> Result<Vec<Deploy>, DbError> {
        deploys::get_latest_deploys_for_apps(&self.pool, app_ids).await
    }

    async fn update_deploy_status(
        &self,
        id: &str,
        status: &str,
        log: Option<&str>,
    ) -> Result<(), DbError> {
        deploys::update_deploy_status(&self.pool, id, status, log).await
    }

    // --- Managed Databases ---

    async fn create_managed_db(&self, db: &NewManagedDatabase) -> Result<ManagedDatabase, DbError> {
        databases::create_managed_db(&self.pool, &self.encryptor, db).await
    }

    async fn update_managed_db_credentials(
        &self,
        id: &str,
        credentials_json: &str,
        container_id: &str,
    ) -> Result<(), DbError> {
        databases::update_managed_db_credentials(
            &self.pool,
            &self.encryptor,
            id,
            credentials_json,
            container_id,
        )
        .await
    }

    async fn list_managed_dbs(&self) -> Result<Vec<ManagedDatabase>, DbError> {
        databases::list_managed_dbs(&self.pool, &self.encryptor).await
    }

    async fn list_managed_dbs_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ManagedDatabase>, DbError> {
        databases::list_managed_dbs_by_project(&self.pool, &self.encryptor, project_id).await
    }

    async fn delete_managed_db(&self, id: &str) -> Result<(), DbError> {
        databases::delete_managed_db(&self.pool, id).await
    }

    // --- Domains ---

    async fn add_domain(&self, domain: &NewDomain) -> Result<Domain, DbError> {
        domains::add_domain(&self.pool, domain).await
    }

    async fn list_domains(&self, app_id: &str) -> Result<Vec<Domain>, DbError> {
        domains::list_domains(&self.pool, app_id).await
    }

    async fn update_domain_status(
        &self,
        id: &str,
        verified: bool,
        ssl_status: &str,
    ) -> Result<(), DbError> {
        domains::update_domain_status(&self.pool, id, verified, ssl_status).await
    }

    async fn delete_domain(&self, id: &str) -> Result<(), DbError> {
        domains::delete_domain(&self.pool, id).await
    }

    // --- Users ---

    async fn create_user(&self, user: &NewUser) -> Result<User, DbError> {
        users::create_user(&self.pool, user).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, DbError> {
        users::get_user_by_email(&self.pool, email).await
    }

    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, DbError> {
        users::get_user_by_id(&self.pool, id).await
    }

    async fn update_user_totp_secret(
        &self,
        user_id: &str,
        secret: Option<&str>,
    ) -> Result<(), DbError> {
        users::update_user_totp_secret(&self.pool, user_id, secret).await
    }

    async fn enable_user_totp(&self, user_id: &str, backup_codes: &str) -> Result<(), DbError> {
        users::enable_user_totp(&self.pool, user_id, backup_codes).await
    }

    async fn disable_user_totp(&self, user_id: &str) -> Result<(), DbError> {
        users::disable_user_totp(&self.pool, user_id).await
    }

    async fn update_user_backup_codes(
        &self,
        user_id: &str,
        backup_codes: &str,
    ) -> Result<(), DbError> {
        users::update_user_backup_codes(&self.pool, user_id, backup_codes).await
    }

    async fn list_users(&self) -> Result<Vec<User>, DbError> {
        users::list_users(&self.pool).await
    }

    // --- User Profile Updates ---

    async fn update_user_password(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> Result<(), DbError> {
        users::update_user_password(&self.pool, user_id, password_hash).await
    }

    async fn update_user_email(&self, user_id: &str, email: &str) -> Result<(), DbError> {
        users::update_user_email(&self.pool, user_id, email).await
    }

    // --- Server Metrics (legacy single-server) ---

    async fn insert_server_metric(
        &self,
        snapshot: &crate::api::routes::server::ServerMetricsSnapshot,
    ) -> Result<(), DbError> {
        servers::insert_server_metric(&self.pool, snapshot).await
    }

    async fn query_server_metrics(
        &self,
        from: &str,
        to: &str,
        limit: usize,
    ) -> Result<Vec<crate::api::routes::server::ServerMetricsSnapshot>, DbError> {
        servers::query_server_metrics(&self.pool, from, to, limit).await
    }

    async fn prune_server_metrics(&self, older_than: &str) -> Result<u64, DbError> {
        maintenance::prune_server_metrics(&self.pool, older_than).await
    }

    // --- Health Checks ---

    async fn create_health_check(&self, hc: &NewHealthCheck) -> Result<HealthCheck, DbError> {
        health::create_health_check(&self.pool, hc).await
    }

    async fn get_health_checks(&self, app_id: &str) -> Result<Vec<HealthCheck>, DbError> {
        health::get_health_checks(&self.pool, app_id).await
    }

    async fn record_health_event(&self, event: &NewHealthCheckEvent) -> Result<(), DbError> {
        health::record_health_event(&self.pool, event).await
    }

    async fn get_health_events(
        &self,
        health_check_id: &str,
        limit: i64,
    ) -> Result<Vec<HealthCheckEvent>, DbError> {
        health::get_health_events(&self.pool, health_check_id, limit).await
    }

    async fn get_health_events_for_checks(
        &self,
        health_check_ids: &[String],
        limit_per_check: i64,
    ) -> Result<Vec<HealthCheckEvent>, DbError> {
        health::get_health_events_for_checks(&self.pool, health_check_ids, limit_per_check).await
    }

    // --- Notifications ---

    async fn create_notification_channel(
        &self,
        channel: &NewNotification,
    ) -> Result<Notification, DbError> {
        notifications::create_notification_channel(&self.pool, &self.encryptor, channel).await
    }

    async fn list_notification_channels(&self) -> Result<Vec<Notification>, DbError> {
        notifications::list_notification_channels(&self.pool, &self.encryptor).await
    }

    async fn create_notification_rule(
        &self,
        rule: &NewNotificationRule,
    ) -> Result<NotificationRule, DbError> {
        notifications::create_notification_rule(&self.pool, rule).await
    }

    async fn get_notification_rules(&self, app_id: &str) -> Result<Vec<NotificationRule>, DbError> {
        notifications::get_notification_rules(&self.pool, app_id).await
    }

    // --- Lookup helpers ---

    async fn get_app_by_repo(&self, repo_url: &str) -> Result<Option<App>, DbError> {
        deploys::get_app_by_repo(&self.pool, repo_url).await
    }

    async fn get_environment_by_branch(
        &self,
        app_id: &str,
        branch: &str,
    ) -> Result<Option<Environment>, DbError> {
        environments::get_environment_by_branch(&self.pool, app_id, branch).await
    }

    // --- Deploy extras ---

    async fn update_deploy_container_id(
        &self,
        deploy_id: &str,
        container_id: &str,
    ) -> Result<(), DbError> {
        deploys::update_deploy_container_id(&self.pool, deploy_id, container_id).await
    }

    async fn update_deploy_image_ref(
        &self,
        deploy_id: &str,
        image_ref: &str,
    ) -> Result<(), DbError> {
        deploys::update_deploy_image_ref(&self.pool, deploy_id, image_ref).await
    }

    async fn update_deploy_env_snapshot(
        &self,
        deploy_id: &str,
        env_snapshot: &str,
    ) -> Result<(), DbError> {
        deploys::update_deploy_env_snapshot(&self.pool, deploy_id, env_snapshot).await
    }

    // --- Env var extras ---

    async fn delete_env_vars_by_environment(&self, environment_id: &str) -> Result<(), DbError> {
        environments::delete_env_vars_by_environment(&self.pool, environment_id).await
    }

    // --- Sessions ---

    async fn create_session(&self, user_id: &str, expires_at: &str) -> Result<Session, DbError> {
        sessions::create_session(&self.pool, user_id, expires_at).await
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, DbError> {
        sessions::get_session(&self.pool, session_id).await
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), DbError> {
        sessions::delete_session(&self.pool, session_id).await
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<(), DbError> {
        sessions::delete_user_sessions(&self.pool, user_id).await
    }

    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, DbError> {
        sessions::list_user_sessions(&self.pool, user_id).await
    }

    async fn delete_user_sessions_except(
        &self,
        user_id: &str,
        keep_session_id: &str,
    ) -> Result<(), DbError> {
        sessions::delete_user_sessions_except(&self.pool, user_id, keep_session_id).await
    }

    // --- API Tokens ---

    async fn create_api_token(
        &self,
        user_id: &str,
        name: &str,
        token_hash: &str,
        expires_at: Option<&str>,
    ) -> Result<ApiToken, DbError> {
        sessions::create_api_token(&self.pool, user_id, name, token_hash, expires_at).await
    }

    async fn get_api_token_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, DbError> {
        sessions::get_api_token_by_hash(&self.pool, token_hash).await
    }

    async fn list_api_tokens(&self, user_id: &str) -> Result<Vec<ApiToken>, DbError> {
        sessions::list_api_tokens(&self.pool, user_id).await
    }

    async fn delete_api_token(&self, id: &str) -> Result<(), DbError> {
        sessions::delete_api_token(&self.pool, id).await
    }

    async fn update_token_last_used(&self, id: &str) -> Result<(), DbError> {
        sessions::update_token_last_used(&self.pool, id).await
    }

    // --- Invitations ---

    async fn create_invitation(
        &self,
        email: &str,
        role: &str,
        token: &str,
        expires_at: &str,
    ) -> Result<Invitation, DbError> {
        sessions::create_invitation(&self.pool, email, role, token, expires_at).await
    }

    async fn get_invitation_by_token(&self, token: &str) -> Result<Option<Invitation>, DbError> {
        sessions::get_invitation_by_token(&self.pool, token).await
    }

    async fn delete_invitation(&self, id: &str) -> Result<(), DbError> {
        sessions::delete_invitation(&self.pool, id).await
    }

    // --- Onboarding ---

    async fn get_onboarding(
        &self,
    ) -> Result<Option<(String, String, String, Option<String>)>, DbError> {
        onboarding::get_onboarding(&self.pool).await
    }

    async fn create_onboarding(&self, started_at: &str) -> Result<(), DbError> {
        onboarding::create_onboarding(&self.pool, started_at).await
    }

    async fn update_onboarding_state(
        &self,
        current_step: &str,
        completed_steps: &str,
        completed_at: Option<&str>,
    ) -> Result<(), DbError> {
        onboarding::update_onboarding_state(&self.pool, current_step, completed_steps, completed_at)
            .await
    }

    // --- Instance Backup ---

    async fn get_instance_backup_config(&self) -> Result<Option<InstanceBackupConfig>, DbError> {
        backups::get_instance_backup_config(&self.pool).await
    }

    async fn upsert_instance_backup_config(
        &self,
        enabled: bool,
        cron_schedule: &str,
        retention_count: i64,
    ) -> Result<InstanceBackupConfig, DbError> {
        backups::upsert_instance_backup_config(&self.pool, enabled, cron_schedule, retention_count)
            .await
    }

    async fn create_instance_backup_record(
        &self,
        filename: &str,
        s3_key: Option<&str>,
    ) -> Result<InstanceBackupRecord, DbError> {
        backups::create_instance_backup_record(&self.pool, filename, s3_key).await
    }

    async fn update_instance_backup_record(
        &self,
        id: &str,
        status: &str,
        size_bytes: i64,
        error_message: Option<&str>,
    ) -> Result<(), DbError> {
        backups::update_instance_backup_record(&self.pool, id, status, size_bytes, error_message)
            .await
    }

    async fn list_instance_backup_history(
        &self,
        limit: i64,
    ) -> Result<Vec<InstanceBackupRecord>, DbError> {
        backups::list_instance_backup_history(&self.pool, limit).await
    }

    async fn delete_instance_backup_record(&self, id: &str) -> Result<(), DbError> {
        backups::delete_instance_backup_record(&self.pool, id).await
    }

    // --- OAuth Identities ---

    async fn create_oauth_identity(
        &self,
        user_id: &str,
        provider: &str,
        provider_user_id: &str,
        provider_email: Option<&str>,
    ) -> Result<OAuthIdentity, DbError> {
        oauth::create_oauth_identity(
            &self.pool,
            user_id,
            provider,
            provider_user_id,
            provider_email,
        )
        .await
    }

    async fn get_oauth_identity(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> Result<Option<OAuthIdentity>, DbError> {
        oauth::get_oauth_identity(&self.pool, provider, provider_user_id).await
    }

    async fn list_oauth_identities_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<OAuthIdentity>, DbError> {
        oauth::list_oauth_identities_for_user(&self.pool, user_id).await
    }

    async fn delete_oauth_identity(&self, id: &str) -> Result<(), DbError> {
        oauth::delete_oauth_identity(&self.pool, id).await
    }

    // --- OAuth Settings ---

    async fn get_oauth_settings(&self) -> Result<Option<OAuthSettings>, DbError> {
        oauth::get_oauth_settings(&self.pool, &self.encryptor).await
    }

    async fn upsert_oauth_settings(&self, settings: &OAuthSettings) -> Result<(), DbError> {
        oauth::upsert_oauth_settings(&self.pool, &self.encryptor, settings).await
    }

    // --- Registration Settings ---

    async fn get_registration_settings(&self) -> Result<RegistrationSettings, DbError> {
        oauth::get_registration_settings(&self.pool).await
    }

    async fn upsert_registration_settings(
        &self,
        allow_registration: bool,
        allowed_domains: Option<&str>,
        default_role: &str,
    ) -> Result<RegistrationSettings, DbError> {
        oauth::upsert_registration_settings(
            &self.pool,
            allow_registration,
            allowed_domains,
            default_role,
        )
        .await
    }

    // --- User Deletion ---

    async fn delete_user(&self, user_id: &str) -> Result<(), DbError> {
        users::delete_user(&self.pool, user_id).await
    }

    async fn count_admin_users(&self) -> Result<i64, DbError> {
        users::count_admin_users(&self.pool).await
    }

    // --- User Preferences ---

    async fn get_user_preferences(&self, user_id: &str) -> Result<serde_json::Value, DbError> {
        users::get_user_preferences(&self.pool, user_id).await
    }

    async fn update_user_preferences(
        &self,
        user_id: &str,
        preferences: &serde_json::Value,
    ) -> Result<(), DbError> {
        users::update_user_preferences(&self.pool, user_id, preferences).await
    }

    // --- Admin 2FA reset ---

    async fn admin_reset_user_2fa(&self, user_id: &str) -> Result<(), DbError> {
        users::admin_reset_user_2fa(&self.pool, user_id).await
    }

    // --- Audit Log ---

    async fn create_audit_log(&self, entry: &NewAuditLogEntry) -> Result<(), DbError> {
        audit::create_audit_log(&self.pool, entry).await
    }

    async fn list_audit_logs(
        &self,
        server_id: Option<&str>,
        action: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, DbError> {
        audit::list_audit_logs(&self.pool, server_id, action, limit, offset).await
    }

    async fn prune_audit_logs(&self, older_than: &str) -> Result<u64, DbError> {
        audit::prune_audit_logs(&self.pool, older_than).await
    }

    // --- Update State ---

    async fn get_update_state(&self) -> Result<UpdateState, DbError> {
        updates::get_update_state(&self.pool).await
    }

    async fn set_update_available(
        &self,
        version: &str,
        release_url: &str,
        release_notes: &str,
        highlights: &str,
    ) -> Result<(), DbError> {
        updates::set_update_available(&self.pool, version, release_url, release_notes, highlights)
            .await
    }

    async fn set_update_download_state(
        &self,
        state: &str,
        progress: i64,
        path: Option<&str>,
    ) -> Result<(), DbError> {
        updates::set_update_download_state(&self.pool, state, progress, path).await
    }

    async fn set_update_error(&self, error: &str) -> Result<(), DbError> {
        updates::set_update_error(&self.pool, error).await
    }

    async fn clear_update_available(&self) -> Result<(), DbError> {
        updates::clear_update_available(&self.pool).await
    }

    async fn update_highest_seen(&self, version: &str) -> Result<(), DbError> {
        updates::update_highest_seen(&self.pool, version).await
    }

    async fn set_last_check_at(&self, timestamp: &str) -> Result<(), DbError> {
        updates::set_last_check_at(&self.pool, timestamp).await
    }

    // --- Update Preferences ---

    async fn set_update_channel(&self, channel: &str) -> Result<(), DbError> {
        updates::set_update_channel(&self.pool, channel).await
    }

    async fn set_auto_update_settings(
        &self,
        enabled: bool,
        channel: &str,
        window_start: &str,
        window_end: &str,
        notify_before_minutes: i64,
    ) -> Result<(), DbError> {
        updates::set_auto_update_settings(
            &self.pool,
            enabled,
            channel,
            window_start,
            window_end,
            notify_before_minutes,
        )
        .await
    }

    async fn set_auto_update_pre_downloaded(&self, pre_downloaded: bool) -> Result<(), DbError> {
        updates::set_auto_update_pre_downloaded(&self.pool, pre_downloaded).await
    }

    async fn has_active_deploys(&self) -> Result<bool, DbError> {
        deploys::has_active_deploys(&self.pool).await
    }

    // --- Skipped Versions ---

    async fn skip_update_version(&self, version: &str) -> Result<(), DbError> {
        updates::skip_update_version(&self.pool, version).await
    }

    async fn is_version_skipped(&self, version: &str) -> Result<bool, DbError> {
        updates::is_version_skipped(&self.pool, version).await
    }

    // --- Update History ---

    async fn record_update_history(&self, entry: &UpdateHistoryEntry) -> Result<(), DbError> {
        updates::record_update_history(&self.pool, entry).await
    }

    async fn list_update_history(&self, limit: usize) -> Result<Vec<UpdateHistoryEntry>, DbError> {
        updates::list_update_history(&self.pool, limit).await
    }

    // --- Servers ---

    async fn create_server(&self, server: &NewServer) -> Result<Server, DbError> {
        servers::create_server(&self.pool, server).await
    }

    async fn get_server(&self, id: &str) -> Result<Option<Server>, DbError> {
        servers::get_server(&self.pool, id).await
    }

    async fn get_server_by_token_hash(&self, hash: &str) -> Result<Option<Server>, DbError> {
        servers::get_server_by_token_hash(&self.pool, hash).await
    }

    async fn list_servers(&self) -> Result<Vec<Server>, DbError> {
        servers::list_servers(&self.pool).await
    }

    async fn update_server(&self, id: &str, update: &ServerUpdate) -> Result<Server, DbError> {
        servers::update_server(&self.pool, id, update).await
    }

    async fn delete_server(&self, id: &str) -> Result<(), DbError> {
        servers::delete_server(&self.pool, id).await
    }

    async fn update_server_heartbeat(&self, id: &str) -> Result<(), DbError> {
        servers::update_server_heartbeat(&self.pool, id).await
    }

    async fn update_server_status(&self, id: &str, status: &str) -> Result<(), DbError> {
        servers::update_server_status(&self.pool, id, status).await
    }

    // --- Server Metrics History ---

    async fn insert_server_metrics_record(
        &self,
        record: &NewServerMetricsRecord,
    ) -> Result<ServerMetricsRecord, DbError> {
        servers::insert_server_metrics_record(&self.pool, record).await
    }

    async fn query_server_metrics_history(
        &self,
        server_id: &str,
        from: &str,
        to: &str,
        limit: usize,
    ) -> Result<Vec<ServerMetricsRecord>, DbError> {
        servers::query_server_metrics_history(&self.pool, server_id, from, to, limit).await
    }

    async fn prune_server_metrics_history(&self, older_than: &str) -> Result<u64, DbError> {
        maintenance::prune_server_metrics_history(&self.pool, older_than).await
    }

    // --- Cleanup / Pruning ---

    async fn prune_expired_sessions(&self, older_than: &str) -> Result<u64, DbError> {
        maintenance::prune_expired_sessions(&self.pool, older_than).await
    }

    async fn prune_expired_tokens(&self) -> Result<u64, DbError> {
        maintenance::prune_expired_tokens(&self.pool).await
    }

    async fn prune_expired_invitations(&self) -> Result<u64, DbError> {
        maintenance::prune_expired_invitations(&self.pool).await
    }

    async fn prune_health_check_events(&self, older_than: &str) -> Result<u64, DbError> {
        maintenance::prune_health_check_events(&self.pool, older_than).await
    }

    async fn prune_old_deploys(&self, older_than: &str, keep_per_app: i64) -> Result<u64, DbError> {
        maintenance::prune_old_deploys(&self.pool, older_than, keep_per_app).await
    }

    // --- Backup ---

    async fn vacuum_into(&self, path: &str) -> Result<(), DbError> {
        maintenance::vacuum_into(&self.pool, path).await
    }

    // --- Migrations ---

    async fn run_migrations(&self) -> Result<(), DbError> {
        maintenance::run_migrations(&self.pool).await
    }
}
