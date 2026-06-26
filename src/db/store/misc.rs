use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait MiscStore: Send + Sync {
    // Service templates
    async fn list_service_templates(&self) -> Result<Vec<ServiceTemplate>, DbError>;

    // Cleanup / Pruning
    async fn prune_expired_sessions(&self, older_than: &str) -> Result<u64, DbError>;
    async fn prune_expired_tokens(&self) -> Result<u64, DbError>;
    async fn prune_expired_invitations(&self) -> Result<u64, DbError>;
    async fn prune_health_check_events(&self, older_than: &str) -> Result<u64, DbError>;
    async fn prune_old_deploys(&self, older_than: &str, keep_per_app: i64) -> Result<u64, DbError>;

    // Log drains
    async fn create_log_drain(&self, drain: &NewLogDrain) -> Result<LogDrain, DbError>;
    async fn list_log_drains_for_app(&self, app_id: &str) -> Result<Vec<LogDrain>, DbError>;
    async fn list_global_log_drains(&self) -> Result<Vec<LogDrain>, DbError>;
    async fn update_log_drain(&self, id: &str, drain: &NewLogDrain) -> Result<LogDrain, DbError>;
    async fn delete_log_drain(&self, id: &str) -> Result<(), DbError>;
    async fn get_log_drain(&self, id: &str) -> Result<Option<LogDrain>, DbError>;

    // Shared variables
    async fn list_shared_variables(
        &self,
        scope: &str,
        scope_id: &str,
    ) -> Result<Vec<SharedVariable>, DbError>;
    async fn set_shared_variable(&self, var: &NewSharedVariable)
        -> Result<SharedVariable, DbError>;
    async fn delete_shared_variable(&self, id: &str) -> Result<(), DbError>;
    async fn get_shared_variables_for_app(
        &self,
        app_id: &str,
    ) -> Result<Vec<SharedVariable>, DbError>;
}
