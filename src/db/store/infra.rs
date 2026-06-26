use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait InfraStore: Send + Sync {
    // SSH keys
    async fn list_ssh_keys(&self, user_id: &str) -> Result<Vec<SshKey>, DbError>;
    async fn create_ssh_key(&self, key: &NewSshKey) -> Result<SshKey, DbError>;
    async fn delete_ssh_key(&self, id: &str) -> Result<(), DbError>;
    async fn get_ssh_key(&self, id: &str) -> Result<Option<SshKey>, DbError>;

    // Container registries
    async fn list_registries(&self) -> Result<Vec<Registry>, DbError>;
    async fn create_registry(&self, reg: &NewRegistry) -> Result<Registry, DbError>;
    async fn delete_registry(&self, id: &str) -> Result<(), DbError>;

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
    async fn update_server_disk_alert_state(&self, id: &str, state: &str) -> Result<(), DbError>;

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

    // Container Metrics History (IF-191 — per-container usage for right-sizing)
    async fn record_container_metrics(
        &self,
        record: &NewContainerMetricsRecord,
    ) -> Result<(), DbError>;
    /// Aggregated per-app usage (avg/peak cpu+mem) over the last `days`.
    async fn container_usage_stats(&self, days: i64) -> Result<Vec<ContainerUsageStats>, DbError>;
    async fn prune_container_metrics(&self, keep_days: i64) -> Result<u64, DbError>;

    // Resource forecasting
    async fn get_server_metrics_for_forecast(
        &self,
        server_id: &str,
        days: i64,
    ) -> Result<Vec<(f64, f64, f64)>, DbError>;
}
