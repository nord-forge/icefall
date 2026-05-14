use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct App {
    pub id: String,
    pub name: String,
    pub git_repo: Option<String>,
    pub git_branch: String,
    pub framework: Option<String>,
    pub build_config: Option<String>,
    pub resource_limits: Option<String>,
    pub preview_enabled: bool,
    pub preview_branch_pattern: Option<String>,
    pub webhook_secret: Option<String>,
    pub tags: Option<String>,
    pub volumes: Option<String>,
    pub image_ref: Option<String>,
    pub compose_content: Option<String>,
    pub project_id: Option<String>,
    pub deploy_mode: String,
    pub server_id: Option<String>,
    pub base_directory: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewApp {
    pub name: String,
    pub git_repo: Option<String>,
    pub git_branch: String,
    pub framework: Option<String>,
    pub image_ref: Option<String>,
    pub compose_content: Option<String>,
    pub deploy_mode: Option<String>,
    pub server_id: Option<String>,
}

#[derive(Default)]
pub struct UpdateApp {
    pub name: Option<String>,
    pub git_repo: Option<String>,
    pub git_branch: Option<String>,
    pub framework: Option<String>,
    pub build_config: Option<String>,
    pub resource_limits: Option<String>,
    pub preview_enabled: Option<bool>,
    pub preview_branch_pattern: Option<Option<String>>,
    pub tags: Option<String>,
    pub volumes: Option<String>,
    pub image_ref: Option<Option<String>>,
    pub compose_content: Option<Option<String>>,
    pub project_id: Option<Option<String>>,
    pub deploy_mode: Option<String>,
    pub server_id: Option<Option<String>>,
    pub base_directory: Option<Option<String>>,
}
