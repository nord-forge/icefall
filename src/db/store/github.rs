use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait GitHubStore: Send + Sync {
    // GitHub installations
    async fn create_github_installation(
        &self,
        installation_id: i64,
        account_login: &str,
        account_type: &str,
    ) -> Result<GitHubInstallation, DbError>;
    async fn list_github_installations(&self) -> Result<Vec<GitHubInstallation>, DbError>;
    async fn delete_github_installation(&self, id: &str) -> Result<(), DbError>;
    async fn get_github_installation(
        &self,
        id: &str,
    ) -> Result<Option<GitHubInstallation>, DbError>;
    async fn get_github_installation_by_installation_id(
        &self,
        installation_id: i64,
    ) -> Result<Option<GitHubInstallation>, DbError>;
    async fn update_github_installation_token(
        &self,
        installation_id: i64,
        access_token: &str,
        token_expires_at: &str,
    ) -> Result<(), DbError>;
    async fn list_installations_needing_token_refresh(
        &self,
        threshold: &str,
    ) -> Result<Vec<GitHubInstallation>, DbError>;

    // GitHub PR comments (preview-env status)
    async fn get_github_pr_comment(
        &self,
        app_id: &str,
        pr_number: i64,
    ) -> Result<Option<GitHubPrComment>, DbError>;
    async fn upsert_github_pr_comment(
        &self,
        app_id: &str,
        installation_id: i64,
        repo_full_name: &str,
        pr_number: i64,
        comment_id: i64,
    ) -> Result<(), DbError>;

    // GitHub Apps
    async fn create_github_app(&self, app: &GitHubApp) -> Result<GitHubApp, DbError>;
    async fn get_github_app(&self, id: &str) -> Result<Option<GitHubApp>, DbError>;
    async fn list_github_apps(&self) -> Result<Vec<GitHubApp>, DbError>;
    async fn delete_github_app(&self, id: &str) -> Result<(), DbError>;
    async fn update_github_installation_app_id(
        &self,
        installation_id: i64,
        github_app_id: &str,
    ) -> Result<(), DbError>;
    async fn get_github_app_for_installation(
        &self,
        installation_id: i64,
    ) -> Result<Option<GitHubApp>, DbError>;
}
