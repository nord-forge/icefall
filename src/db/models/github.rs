use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubInstallation {
    pub id: String,
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub token_expires_at: Option<String>,
    pub github_app_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubApp {
    pub id: String,
    pub name: String,
    pub app_id: i64,
    pub client_id: String,
    #[serde(skip_serializing)]
    pub client_secret: String,
    #[serde(skip_serializing)]
    pub private_key: String,
    #[serde(skip_serializing)]
    pub webhook_secret: String,
    pub html_url: String,
    pub api_url: String,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
}
