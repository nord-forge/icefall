use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait ObservabilityStore: Send + Sync {
    // Config history
    async fn record_config_change(
        &self,
        resource_type: &str,
        resource_id: &str,
        field: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
        changed_by: Option<&str>,
    ) -> Result<(), DbError>;
    async fn list_config_history(
        &self,
        resource_type: &str,
        resource_id: &str,
        limit: i64,
    ) -> Result<Vec<ConfigHistoryEntry>, DbError>;

    // Drift events
    async fn record_drift_event(
        &self,
        app_id: &str,
        drifted_fields: &str,
        declared: Option<&str>,
        actual: Option<&str>,
    ) -> Result<DriftEvent, DbError>;
    async fn list_drift_events(&self, app_id: &str, limit: i64)
        -> Result<Vec<DriftEvent>, DbError>;
    async fn resolve_drift_event(&self, id: &str) -> Result<(), DbError>;

    // Incidents
    async fn create_incident(&self, incident: &NewIncident) -> Result<Incident, DbError>;
    async fn list_incidents(&self, limit: i64) -> Result<Vec<Incident>, DbError>;
    async fn update_incident_status(&self, id: &str, status: &str) -> Result<(), DbError>;
    async fn add_incident_note(
        &self,
        incident_id: &str,
        content: &str,
        author_id: Option<&str>,
    ) -> Result<IncidentNote, DbError>;

    // Health Checks
    async fn create_health_check(&self, hc: &NewHealthCheck) -> Result<HealthCheck, DbError>;
    async fn get_health_checks(&self, app_id: &str) -> Result<Vec<HealthCheck>, DbError>;
    async fn update_health_check(
        &self,
        id: &str,
        interval_secs: Option<i64>,
        failure_threshold: Option<i64>,
        auto_restart: Option<bool>,
        config: Option<&str>,
    ) -> Result<(), DbError>;
    async fn delete_health_check(&self, id: &str) -> Result<(), DbError>;
    async fn record_health_event(&self, event: &NewHealthCheckEvent) -> Result<(), DbError>;
    async fn get_health_events(
        &self,
        health_check_id: &str,
        limit: i64,
    ) -> Result<Vec<HealthCheckEvent>, DbError>;
    async fn get_health_events_for_checks(
        &self,
        health_check_ids: &[String],
        limit_per_check: i64,
    ) -> Result<Vec<HealthCheckEvent>, DbError>;

    // Search
    async fn search(&self, query: &str) -> Result<serde_json::Value, DbError>;

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
}
