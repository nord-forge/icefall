use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait ProjectStore: Send + Sync {
    async fn list_projects(&self) -> Result<Vec<Project>, DbError>;
    async fn create_project(&self, project: &NewProject) -> Result<Project, DbError>;
    async fn get_project(&self, id: &str) -> Result<Option<Project>, DbError>;
    async fn update_project(&self, id: &str, update: &UpdateProject) -> Result<Project, DbError>;
    async fn delete_project(&self, id: &str) -> Result<(), DbError>;
}
