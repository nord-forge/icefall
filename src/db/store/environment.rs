use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait EnvironmentStore: Send + Sync {
    // Environments
    async fn create_environment(&self, env: &NewEnvironment) -> Result<Environment, DbError>;
    async fn list_environments(&self, app_id: &str) -> Result<Vec<Environment>, DbError>;
    async fn delete_environment(&self, id: &str) -> Result<(), DbError>;

    // Env Vars
    async fn set_env_var(&self, env_var: &NewEnvVar) -> Result<EnvVar, DbError>;
    async fn get_env_vars(&self, environment_id: &str) -> Result<Vec<EnvVar>, DbError>;
    async fn delete_env_var(&self, id: &str) -> Result<(), DbError>;

    // Env var extras
    async fn delete_env_vars_by_environment(&self, environment_id: &str) -> Result<(), DbError>;

    // Project environments
    async fn create_project_environment(
        &self,
        env: &NewProjectEnvironment,
    ) -> Result<ProjectEnvironment, DbError>;
    async fn list_project_environments(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectEnvironment>, DbError>;
    async fn update_project_environment(
        &self,
        id: &str,
        name: &str,
        color: Option<&str>,
    ) -> Result<ProjectEnvironment, DbError>;
    async fn delete_project_environment(&self, id: &str) -> Result<(), DbError>;
    async fn get_project_environment(
        &self,
        id: &str,
    ) -> Result<Option<ProjectEnvironment>, DbError>;
}
