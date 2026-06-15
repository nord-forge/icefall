use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubInstallation {
    pub id: String,
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    /// Cached installation access token (decrypted in memory only). Never
    /// serialized to API responses.
    #[serde(skip_serializing)]
    pub access_token: Option<String>,
    pub token_expires_at: Option<String>,
    pub github_app_id: Option<String>,
    pub created_at: String,
}

/// A PR comment Icefall manages (preview-env status). Tracked so it can be
/// edited on each push instead of re-posted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPrComment {
    pub id: String,
    pub app_id: String,
    pub installation_id: i64,
    pub repo_full_name: String,
    pub pr_number: i64,
    pub comment_id: i64,
    pub created_at: String,
    pub updated_at: String,
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
