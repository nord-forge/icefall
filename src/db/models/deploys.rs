use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Deploy {
    pub id: String,
    pub app_id: String,
    pub environment_id: String,
    pub status: String,
    pub git_sha: Option<String>,
    pub build_log: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub image_ref: Option<String>,
    pub container_id: Option<String>,
    pub env_snapshot: Option<String>,
    pub server_id: Option<String>,
    pub tag: Option<String>,
    pub created_at: String,
}

pub struct NewDeploy {
    pub app_id: String,
    pub environment_id: String,
    pub git_sha: Option<String>,
    pub server_id: Option<String>,
    pub tag: Option<String>,
}
