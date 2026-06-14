use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeployApproval {
    pub id: String,
    pub deploy_id: String,
    pub action: String,
    pub user_id: String,
    pub comment: Option<String>,
    pub created_at: String,
}
