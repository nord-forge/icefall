use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Environment {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub env_type: String,
    pub branch: Option<String>,
    pub created_at: String,
}

pub struct NewEnvironment {
    pub app_id: String,
    pub name: String,
    pub env_type: String,
    pub branch: Option<String>,
}
