use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubInstallation {
    pub id: String,
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub token_expires_at: Option<String>,
    pub created_at: String,
}
