use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub icon_url: Option<String>,
    pub categories: Option<String>,
    pub website: Option<String>,
    pub required_inputs: String,
    pub default_env: Option<String>,
    pub min_resources: Option<String>,
    pub compose_content: String,
    pub readme: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
