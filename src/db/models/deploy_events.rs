use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeployEvent {
    pub id: String,
    pub deploy_id: String,
    pub event_type: String,
    pub data: String,
    pub timestamp: String,
}
