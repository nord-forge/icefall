use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait AppStore: Send + Sync {
    // Apps
    async fn create_app(&self, app: &NewApp) -> Result<App, DbError>;
    async fn get_app(&self, id: &str) -> Result<Option<App>, DbError>;
    async fn get_app_by_name(&self, name: &str) -> Result<Option<App>, DbError>;
    async fn list_apps(&self) -> Result<Vec<App>, DbError>;
    async fn list_apps_by_project(&self, project_id: &str) -> Result<Vec<App>, DbError>;
    async fn update_app(&self, id: &str, update: &UpdateApp) -> Result<App, DbError>;
    async fn delete_app(&self, id: &str) -> Result<(), DbError>;
    async fn set_app_webhook_secret(&self, app_id: &str, secret: &str) -> Result<(), DbError>;

    // App instances (multi-instance / load balancing)
    async fn create_app_instance(&self, instance: &NewAppInstance) -> Result<AppInstance, DbError>;
    async fn get_app_instance(&self, id: &str) -> Result<Option<AppInstance>, DbError>;
    async fn list_app_instances(&self, app_id: &str) -> Result<Vec<AppInstance>, DbError>;
    async fn list_app_instances_by_server(
        &self,
        server_id: &str,
    ) -> Result<Vec<AppInstance>, DbError>;
    async fn update_app_instance(
        &self,
        id: &str,
        update: &UpdateAppInstance,
    ) -> Result<AppInstance, DbError>;
    async fn delete_app_instance(&self, id: &str) -> Result<(), DbError>;

    // App cloning
    async fn clone_app(
        &self,
        source_app_id: &str,
        new_name: &str,
        target_project_id: Option<&str>,
        target_server_id: Option<&str>,
    ) -> Result<App, DbError>;

    // Lookup helpers
    async fn get_app_by_repo(&self, repo_url: &str) -> Result<Option<App>, DbError>;
    async fn get_environment_by_branch(
        &self,
        app_id: &str,
        branch: &str,
    ) -> Result<Option<Environment>, DbError>;
}
