use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait DeployStore: Send + Sync {
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
    async fn list_due_scheduled_deploys(&self) -> Result<Vec<Deploy>, DbError>;
    async fn start_scheduled_deploy(&self, deploy_id: &str) -> Result<bool, DbError>;
    async fn reschedule_deploy(&self, deploy_id: &str, scheduled_at: &str)
        -> Result<bool, DbError>;

    // Deploy events
    async fn record_deploy_event(
        &self,
        deploy_id: &str,
        event_type: &str,
        data: &serde_json::Value,
    ) -> Result<(), DbError>;
    async fn list_deploy_events(&self, deploy_id: &str) -> Result<Vec<DeployEvent>, DbError>;

    // Deploy approvals
    async fn create_deploy_approval(
        &self,
        deploy_id: &str,
        action: &str,
        user_id: &str,
        comment: Option<&str>,
    ) -> Result<DeployApproval, DbError>;
    async fn get_deploy_approval(&self, deploy_id: &str)
        -> Result<Option<DeployApproval>, DbError>;

    // Canary results
    #[allow(clippy::too_many_arguments)]
    async fn store_canary_result(
        &self,
        deploy_id: &str,
        p50: f64,
        p95: f64,
        p99: f64,
        errors: i32,
        total: i32,
        verdict: &str,
    ) -> Result<CanaryResult, DbError>;
    async fn get_canary_baseline(&self, app_id: &str) -> Result<Option<CanaryResult>, DbError>;

    // Deploy analytics
    async fn get_deploy_analytics(
        &self,
        from: &str,
        to: &str,
    ) -> Result<serde_json::Value, DbError>;

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
    async fn update_deploy_config_hash(
        &self,
        deploy_id: &str,
        config_hash: &str,
    ) -> Result<(), DbError>;
}
