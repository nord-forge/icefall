mod analytics;
mod apps;
mod audit;
mod backups;
mod canary;
mod cleanup_runs;
mod cleanup_schedule;
mod config_history;
mod databases;
mod deploy_approvals;
mod deploy_events;
mod deploys;
mod domains;
mod drift;
mod environments;
mod forecast;
mod github;
mod health;
mod incidents;
mod instance_lifecycle_tests;
mod log_drains;
mod maintenance;
mod notifications;
mod oauth;
mod onboarding;
mod project_environments;
mod projects;
mod public_ports;
mod registries;
mod search;
mod servers;
mod sessions;
mod shared_variables;
mod ssh_keys;
mod team_isolation_tests;
mod team_scoping;
mod teams;
mod updates;
mod users;
mod webhooks;

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

    async fn create_app_instance(&self, instance: &NewAppInstance) -> Result<AppInstance, DbError> {
        apps::create_app_instance(&self.pool, instance).await
    }

    async fn get_app_instance(&self, id: &str) -> Result<Option<AppInstance>, DbError> {
        apps::get_app_instance(&self.pool, id).await
    }

    async fn list_app_instances(&self, app_id: &str) -> Result<Vec<AppInstance>, DbError> {
        apps::list_app_instances(&self.pool, app_id).await
    }

    async fn list_app_instances_by_server(
        &self,
        server_id: &str,
    ) -> Result<Vec<AppInstance>, DbError> {
        apps::list_app_instances_by_server(&self.pool, server_id).await
    }

    async fn update_app_instance(
        &self,
        id: &str,
        update: &UpdateAppInstance,
    ) -> Result<AppInstance, DbError> {
        apps::update_app_instance(&self.pool, id, update).await
    }

    async fn delete_app_instance(&self, id: &str) -> Result<(), DbError> {
        apps::delete_app_instance(&self.pool, id).await
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

    async fn list_all_domains(&self) -> Result<Vec<Domain>, DbError> {
        domains::list_all_domains(&self.pool).await
    }

    async fn update_domain_ssl_info(
        &self,
        id: &str,
        issuer: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<(), DbError> {
        domains::update_domain_ssl_info(&self.pool, id, issuer, expires_at).await
    }

    // --- Webhook endpoints ---

    async fn list_webhook_endpoints(&self) -> Result<Vec<WebhookEndpoint>, DbError> {
        webhooks::list_webhook_endpoints(&self.pool).await
    }

    async fn create_webhook_endpoint(
        &self,
        endpoint: &NewWebhookEndpoint,
    ) -> Result<WebhookEndpoint, DbError> {
        webhooks::create_webhook_endpoint(&self.pool, endpoint).await
    }

    async fn delete_webhook_endpoint(&self, id: &str) -> Result<(), DbError> {
        webhooks::delete_webhook_endpoint(&self.pool, id).await
    }

    async fn create_webhook_delivery(
        &self,
        endpoint_id: &str,
        event: &str,
        status_code: Option<i32>,
        response_time_ms: Option<i32>,
        attempt: i32,
        error: Option<&str>,
    ) -> Result<(), DbError> {
        webhooks::create_webhook_delivery(
            &self.pool,
            endpoint_id,
            event,
            status_code,
            response_time_ms,
            attempt,
            error,
        )
        .await
    }

    async fn list_webhook_deliveries(
        &self,
        endpoint_id: &str,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, DbError> {
        webhooks::list_webhook_deliveries(&self.pool, endpoint_id, limit).await
    }

    // --- Search ---

    async fn search(&self, query: &str) -> Result<serde_json::Value, DbError> {
        search::search(&self.pool, query).await
    }

    // --- SSH keys ---

    async fn list_ssh_keys(&self, user_id: &str) -> Result<Vec<SshKey>, DbError> {
        ssh_keys::list_ssh_keys(&self.pool, user_id).await
    }

    async fn create_ssh_key(&self, key: &NewSshKey) -> Result<SshKey, DbError> {
        ssh_keys::create_ssh_key(&self.pool, key).await
    }

    async fn delete_ssh_key(&self, id: &str) -> Result<(), DbError> {
        ssh_keys::delete_ssh_key(&self.pool, id).await
    }

    async fn get_ssh_key(&self, id: &str) -> Result<Option<SshKey>, DbError> {
        ssh_keys::get_ssh_key(&self.pool, id).await
    }

    // --- Container registries ---

    async fn list_registries(&self) -> Result<Vec<Registry>, DbError> {
        registries::list_registries(&self.pool, &self.encryptor).await
    }

    async fn create_registry(&self, reg: &NewRegistry) -> Result<Registry, DbError> {
        registries::create_registry(&self.pool, &self.encryptor, reg).await
    }

    async fn delete_registry(&self, id: &str) -> Result<(), DbError> {
        registries::delete_registry(&self.pool, id).await
    }

    // --- Public ports ---

    async fn allocate_public_port(
        &self,
        resource_type: &str,
        resource_id: &str,
        port: i32,
        ip_whitelist: Option<&str>,
    ) -> Result<PublicPort, DbError> {
        public_ports::allocate_public_port(
            &self.pool,
            resource_type,
            resource_id,
            port,
            ip_whitelist,
        )
        .await
    }

    async fn release_public_port(&self, resource_id: &str) -> Result<(), DbError> {
        public_ports::release_public_port(&self.pool, resource_id).await
    }

    async fn get_public_port(&self, resource_id: &str) -> Result<Option<PublicPort>, DbError> {
        public_ports::get_public_port(&self.pool, resource_id).await
    }

    // --- GitHub installations ---

    async fn create_github_installation(
        &self,
        installation_id: i64,
        account_login: &str,
        account_type: &str,
    ) -> Result<GitHubInstallation, DbError> {
        github::create_github_installation(&self.pool, installation_id, account_login, account_type)
            .await
    }

    async fn list_github_installations(&self) -> Result<Vec<GitHubInstallation>, DbError> {
        github::list_github_installations(&self.pool).await
    }

    async fn delete_github_installation(&self, id: &str) -> Result<(), DbError> {
        github::delete_github_installation(&self.pool, id).await
    }

    // --- GitHub Apps ---

    async fn create_github_app(&self, app: &GitHubApp) -> Result<GitHubApp, DbError> {
        github::create_github_app(&self.pool, &self.encryptor, app).await
    }

    async fn get_github_app(&self, id: &str) -> Result<Option<GitHubApp>, DbError> {
        github::get_github_app(&self.pool, &self.encryptor, id).await
    }

    async fn list_github_apps(&self) -> Result<Vec<GitHubApp>, DbError> {
        github::list_github_apps(&self.pool, &self.encryptor).await
    }

    async fn delete_github_app(&self, id: &str) -> Result<(), DbError> {
        github::delete_github_app(&self.pool, id).await
    }

    async fn update_github_installation_app_id(
        &self,
        installation_id: i64,
        github_app_id: &str,
    ) -> Result<(), DbError> {
        github::update_github_installation_app_id(&self.pool, installation_id, github_app_id).await
    }

    async fn get_github_app_for_installation(
        &self,
        installation_id: i64,
    ) -> Result<Option<GitHubApp>, DbError> {
        github::get_github_app_for_installation(&self.pool, &self.encryptor, installation_id).await
    }

    // --- Config history ---

    async fn record_config_change(
        &self,
        resource_type: &str,
        resource_id: &str,
        field: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
        changed_by: Option<&str>,
    ) -> Result<(), DbError> {
        config_history::record_config_change(
            &self.pool,
            resource_type,
            resource_id,
            field,
            old_value,
            new_value,
            changed_by,
        )
        .await
    }

    async fn list_config_history(
        &self,
        resource_type: &str,
        resource_id: &str,
        limit: i64,
    ) -> Result<Vec<ConfigHistoryEntry>, DbError> {
        config_history::list_config_history(&self.pool, resource_type, resource_id, limit).await
    }

    // --- Deploy events ---

    async fn record_deploy_event(
        &self,
        deploy_id: &str,
        event_type: &str,
        data: &serde_json::Value,
    ) -> Result<(), DbError> {
        deploy_events::record_deploy_event(&self.pool, deploy_id, event_type, data).await
    }

    async fn list_deploy_events(&self, deploy_id: &str) -> Result<Vec<DeployEvent>, DbError> {
        deploy_events::list_deploy_events(&self.pool, deploy_id).await
    }

    // --- Deploy approvals ---

    async fn create_deploy_approval(
        &self,
        deploy_id: &str,
        action: &str,
        user_id: &str,
        comment: Option<&str>,
    ) -> Result<DeployApproval, DbError> {
        deploy_approvals::create_deploy_approval(&self.pool, deploy_id, action, user_id, comment)
            .await
    }

    async fn get_deploy_approval(
        &self,
        deploy_id: &str,
    ) -> Result<Option<DeployApproval>, DbError> {
        deploy_approvals::get_deploy_approval(&self.pool, deploy_id).await
    }

    // --- Canary results ---

    async fn store_canary_result(
        &self,
        deploy_id: &str,
        p50: f64,
        p95: f64,
        p99: f64,
        errors: i32,
        total: i32,
        verdict: &str,
    ) -> Result<CanaryResult, DbError> {
        canary::store_canary_result(&self.pool, deploy_id, p50, p95, p99, errors, total, verdict)
            .await
    }

    async fn get_canary_baseline(&self, app_id: &str) -> Result<Option<CanaryResult>, DbError> {
        canary::get_canary_baseline(&self.pool, app_id).await
    }

    // --- Drift events ---

    async fn record_drift_event(
        &self,
        app_id: &str,
        drifted_fields: &str,
        declared: Option<&str>,
        actual: Option<&str>,
    ) -> Result<DriftEvent, DbError> {
        drift::record_drift_event(&self.pool, app_id, drifted_fields, declared, actual).await
    }

    async fn list_drift_events(
        &self,
        app_id: &str,
        limit: i64,
    ) -> Result<Vec<DriftEvent>, DbError> {
        drift::list_drift_events(&self.pool, app_id, limit).await
    }

    async fn resolve_drift_event(&self, id: &str) -> Result<(), DbError> {
        drift::resolve_drift_event(&self.pool, id).await
    }

    // --- Resource forecasting ---

    async fn get_server_metrics_for_forecast(
        &self,
        server_id: &str,
        days: i64,
    ) -> Result<Vec<(f64, f64, f64)>, DbError> {
        forecast::get_server_metrics_for_forecast(&self.pool, server_id, days).await
    }

    // --- Incidents ---

    async fn create_incident(&self, incident: &NewIncident) -> Result<Incident, DbError> {
        incidents::create_incident(&self.pool, incident).await
    }

    async fn list_incidents(&self, limit: i64) -> Result<Vec<Incident>, DbError> {
        incidents::list_incidents(&self.pool, limit).await
    }

    async fn update_incident_status(&self, id: &str, status: &str) -> Result<(), DbError> {
        incidents::update_incident_status(&self.pool, id, status).await
    }

    async fn add_incident_note(
        &self,
        incident_id: &str,
        content: &str,
        author_id: Option<&str>,
    ) -> Result<IncidentNote, DbError> {
        incidents::add_incident_note(&self.pool, incident_id, content, author_id).await
    }

    // --- Deploy analytics ---

    async fn get_deploy_analytics(
        &self,
        from: &str,
        to: &str,
    ) -> Result<serde_json::Value, DbError> {
        analytics::get_deploy_analytics(&self.pool, from, to).await
    }

    // --- Service templates ---

    async fn list_service_templates(&self) -> Result<Vec<ServiceTemplate>, DbError> {
        let templates = sqlx::query_as::<_, ServiceTemplate>(
            "SELECT * FROM service_templates ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(templates)
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

    async fn update_health_check(
        &self,
        id: &str,
        interval_secs: Option<i64>,
        failure_threshold: Option<i64>,
        auto_restart: Option<bool>,
        config: Option<&str>,
    ) -> Result<(), DbError> {
        health::update_health_check(
            &self.pool,
            id,
            interval_secs,
            failure_threshold,
            auto_restart,
            config,
        )
        .await
    }

    async fn delete_health_check(&self, id: &str) -> Result<(), DbError> {
        health::delete_health_check(&self.pool, id).await
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

    async fn update_deploy_config_hash(
        &self,
        deploy_id: &str,
        config_hash: &str,
    ) -> Result<(), DbError> {
        deploys::update_deploy_config_hash(&self.pool, deploy_id, config_hash).await
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
        team_id: Option<&str>,
    ) -> Result<ApiToken, DbError> {
        sessions::create_api_token(&self.pool, user_id, name, token_hash, expires_at, team_id).await
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

    async fn update_server_disk_alert_state(&self, id: &str, state: &str) -> Result<(), DbError> {
        servers::update_server_disk_alert_state(&self.pool, id, state).await
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

    // --- Log Drains ---

    async fn create_log_drain(&self, drain: &NewLogDrain) -> Result<LogDrain, DbError> {
        log_drains::create_log_drain(&self.pool, drain).await
    }

    async fn list_log_drains_for_app(&self, app_id: &str) -> Result<Vec<LogDrain>, DbError> {
        log_drains::list_log_drains_for_app(&self.pool, app_id).await
    }

    async fn list_global_log_drains(&self) -> Result<Vec<LogDrain>, DbError> {
        log_drains::list_global_log_drains(&self.pool).await
    }

    async fn update_log_drain(&self, id: &str, drain: &NewLogDrain) -> Result<LogDrain, DbError> {
        log_drains::update_log_drain(&self.pool, id, drain).await
    }

    async fn delete_log_drain(&self, id: &str) -> Result<(), DbError> {
        log_drains::delete_log_drain(&self.pool, id).await
    }

    async fn get_log_drain(&self, id: &str) -> Result<Option<LogDrain>, DbError> {
        log_drains::get_log_drain(&self.pool, id).await
    }

    // --- Project Environments ---

    async fn create_project_environment(
        &self,
        env: &NewProjectEnvironment,
    ) -> Result<ProjectEnvironment, DbError> {
        project_environments::create_project_environment(&self.pool, env).await
    }

    async fn list_project_environments(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectEnvironment>, DbError> {
        project_environments::list_project_environments(&self.pool, project_id).await
    }

    async fn update_project_environment(
        &self,
        id: &str,
        name: &str,
        color: Option<&str>,
    ) -> Result<ProjectEnvironment, DbError> {
        project_environments::update_project_environment(&self.pool, id, name, color).await
    }

    async fn delete_project_environment(&self, id: &str) -> Result<(), DbError> {
        project_environments::delete_project_environment(&self.pool, id).await
    }

    async fn get_project_environment(
        &self,
        id: &str,
    ) -> Result<Option<ProjectEnvironment>, DbError> {
        project_environments::get_project_environment(&self.pool, id).await
    }

    // --- Cleanup Schedule ---

    async fn get_cleanup_schedule(&self) -> Result<Option<CleanupSchedule>, DbError> {
        cleanup_schedule::get_cleanup_schedule(&self.pool).await
    }

    async fn upsert_cleanup_schedule(
        &self,
        schedule: &CleanupSchedule,
    ) -> Result<CleanupSchedule, DbError> {
        cleanup_schedule::upsert_cleanup_schedule(&self.pool, schedule).await
    }

    // --- Cleanup Runs ---

    async fn create_cleanup_run(&self) -> Result<CleanupRun, DbError> {
        cleanup_runs::create_cleanup_run(&self.pool).await
    }

    async fn finish_cleanup_run(
        &self,
        id: &str,
        status: &str,
        freed_bytes: i64,
        removed_items: i64,
        error: Option<&str>,
        details: Option<&str>,
    ) -> Result<(), DbError> {
        cleanup_runs::finish_cleanup_run(
            &self.pool,
            id,
            status,
            freed_bytes,
            removed_items,
            error,
            details,
        )
        .await
    }

    async fn list_cleanup_runs(&self, limit: i64) -> Result<Vec<CleanupRun>, DbError> {
        cleanup_runs::list_cleanup_runs(&self.pool, limit).await
    }

    // --- Shared Variables ---

    async fn list_shared_variables(
        &self,
        scope: &str,
        scope_id: &str,
    ) -> Result<Vec<SharedVariable>, DbError> {
        shared_variables::list_shared_variables(&self.pool, &self.encryptor, scope, scope_id).await
    }

    async fn set_shared_variable(
        &self,
        var: &NewSharedVariable,
    ) -> Result<SharedVariable, DbError> {
        shared_variables::set_shared_variable(&self.pool, &self.encryptor, var).await
    }

    async fn delete_shared_variable(&self, id: &str) -> Result<(), DbError> {
        shared_variables::delete_shared_variable(&self.pool, id).await
    }

    async fn get_shared_variables_for_app(
        &self,
        app_id: &str,
    ) -> Result<Vec<SharedVariable>, DbError> {
        shared_variables::get_shared_variables_for_app(&self.pool, &self.encryptor, app_id).await
    }

    // --- Team-scoped queries ---

    async fn list_apps_by_team(&self, team_id: &str) -> Result<Vec<App>, DbError> {
        team_scoping::list_apps_by_team(&self.pool, team_id).await
    }

    async fn list_projects_by_team(&self, team_id: &str) -> Result<Vec<Project>, DbError> {
        team_scoping::list_projects_by_team(&self.pool, team_id).await
    }

    async fn list_managed_dbs_by_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<ManagedDatabase>, DbError> {
        team_scoping::list_managed_dbs_by_team(&self.pool, &self.encryptor, team_id).await
    }

    async fn list_ssh_keys_by_team(&self, team_id: &str) -> Result<Vec<SshKey>, DbError> {
        team_scoping::list_ssh_keys_by_team(&self.pool, team_id).await
    }

    async fn list_registries_by_team(&self, team_id: &str) -> Result<Vec<Registry>, DbError> {
        team_scoping::list_registries_by_team(&self.pool, &self.encryptor, team_id).await
    }

    async fn list_notification_channels_by_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<Notification>, DbError> {
        team_scoping::list_notification_channels_by_team(&self.pool, &self.encryptor, team_id).await
    }

    async fn list_api_tokens_by_team(&self, team_id: &str) -> Result<Vec<ApiToken>, DbError> {
        team_scoping::list_api_tokens_by_team(&self.pool, team_id).await
    }

    async fn set_app_team(&self, app_id: &str, team_id: &str) -> Result<(), DbError> {
        team_scoping::set_app_team(&self.pool, app_id, team_id).await
    }

    async fn set_project_team(&self, project_id: &str, team_id: &str) -> Result<(), DbError> {
        team_scoping::set_project_team(&self.pool, project_id, team_id).await
    }

    async fn set_database_team(&self, db_id: &str, team_id: &str) -> Result<(), DbError> {
        team_scoping::set_database_team(&self.pool, db_id, team_id).await
    }

    // --- Teams ---

    async fn create_team(&self, team: &NewTeam) -> Result<Team, DbError> {
        teams::create_team(&self.pool, team).await
    }

    async fn get_team(&self, id: &str) -> Result<Option<Team>, DbError> {
        teams::get_team(&self.pool, id).await
    }

    async fn get_team_by_slug(&self, slug: &str) -> Result<Option<Team>, DbError> {
        teams::get_team_by_slug(&self.pool, slug).await
    }

    async fn list_teams_for_user(&self, user_id: &str) -> Result<Vec<Team>, DbError> {
        teams::list_teams_for_user(&self.pool, user_id).await
    }

    async fn update_team(&self, id: &str, update: &UpdateTeam) -> Result<Team, DbError> {
        teams::update_team(&self.pool, id, update).await
    }

    async fn delete_team(&self, id: &str) -> Result<(), DbError> {
        teams::delete_team(&self.pool, id).await
    }

    async fn count_team_resources(&self, team_id: &str) -> Result<i64, DbError> {
        teams::count_team_resources(&self.pool, team_id).await
    }

    async fn add_team_member(
        &self,
        team_id: &str,
        user_id: &str,
        role: &str,
        invited_by: Option<&str>,
    ) -> Result<TeamMembership, DbError> {
        teams::add_team_member(&self.pool, team_id, user_id, role, invited_by).await
    }

    async fn list_team_members(&self, team_id: &str) -> Result<Vec<TeamMember>, DbError> {
        teams::list_team_members(&self.pool, team_id).await
    }

    async fn get_team_membership(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<Option<TeamMembership>, DbError> {
        teams::get_team_membership(&self.pool, team_id, user_id).await
    }

    async fn update_team_member_role(
        &self,
        team_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), DbError> {
        teams::update_team_member_role(&self.pool, team_id, user_id, role).await
    }

    async fn remove_team_member(&self, team_id: &str, user_id: &str) -> Result<(), DbError> {
        teams::remove_team_member(&self.pool, team_id, user_id).await
    }

    async fn create_team_invitation(
        &self,
        team_id: &str,
        email: &str,
        role: &str,
        token: &str,
        invited_by: &str,
        expires_at: &str,
    ) -> Result<TeamInvitation, DbError> {
        teams::create_team_invitation(
            &self.pool, team_id, email, role, token, invited_by, expires_at,
        )
        .await
    }

    async fn get_team_invitation_by_token(
        &self,
        token: &str,
    ) -> Result<Option<TeamInvitation>, DbError> {
        teams::get_team_invitation_by_token(&self.pool, token).await
    }

    async fn list_team_invitations(&self, team_id: &str) -> Result<Vec<TeamInvitation>, DbError> {
        teams::list_team_invitations(&self.pool, team_id).await
    }

    async fn delete_team_invitation(&self, id: &str) -> Result<(), DbError> {
        teams::delete_team_invitation(&self.pool, id).await
    }

    async fn set_session_team(&self, session_id: &str, team_id: &str) -> Result<(), DbError> {
        teams::set_session_team(&self.pool, session_id, team_id).await
    }

    // --- Cross-team server sharing ---

    async fn share_server_with_team(
        &self,
        server_id: &str,
        team_id: &str,
        access_level: &str,
        granted_by: &str,
    ) -> Result<ServerTeamAccess, DbError> {
        teams::share_server_with_team(&self.pool, server_id, team_id, access_level, granted_by)
            .await
    }

    async fn revoke_server_share(&self, server_id: &str, team_id: &str) -> Result<(), DbError> {
        teams::revoke_server_share(&self.pool, server_id, team_id).await
    }

    async fn list_server_shares(&self, server_id: &str) -> Result<Vec<ServerTeamAccess>, DbError> {
        teams::list_server_shares(&self.pool, server_id).await
    }

    async fn list_servers_shared_with_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<(Server, String)>, DbError> {
        teams::list_servers_shared_with_team(&self.pool, team_id).await
    }

    // --- Migrations ---

    async fn run_migrations(&self) -> Result<(), DbError> {
        maintenance::run_migrations(&self.pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_https() {
        assert_eq!(
            normalize_repo_url("https://github.com/user/repo"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn normalize_strips_http() {
        assert_eq!(
            normalize_repo_url("http://github.com/user/repo"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn normalize_strips_git_suffix() {
        assert_eq!(
            normalize_repo_url("https://github.com/user/repo.git"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn normalize_strips_trailing_slash() {
        assert_eq!(
            normalize_repo_url("https://github.com/user/repo/"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn normalize_converts_ssh_to_path() {
        assert_eq!(
            normalize_repo_url("git@github.com:user/repo.git"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn normalize_lowercases() {
        assert_eq!(
            normalize_repo_url("https://GitHub.COM/User/Repo"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(
            normalize_repo_url("  https://github.com/user/repo  "),
            "github.com/user/repo"
        );
    }

    #[test]
    fn normalize_handles_combined() {
        assert_eq!(
            normalize_repo_url("git@gitlab.com:org/project.git/"),
            "gitlab.com/org/project"
        );
    }

    #[test]
    fn normalize_ssh_and_https_produce_same_result() {
        let https = normalize_repo_url("https://github.com/org/repo.git");
        let ssh = normalize_repo_url("git@github.com:org/repo.git");
        assert_eq!(https, ssh);
    }

    #[test]
    fn normalize_self_hosted_gitlab_url() {
        assert_eq!(
            normalize_repo_url("https://git.example.com/team/project"),
            "git.example.com/team/project"
        );
    }

    #[test]
    fn normalize_deeply_nested_repo_path() {
        assert_eq!(
            normalize_repo_url("https://github.com/org/sub-group/repo.git"),
            "github.com/org/sub-group/repo"
        );
    }
}
