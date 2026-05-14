use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DriftEvent {
    pub id: String,
    pub app_id: String,
    pub drifted_fields: String,
    pub declared_state: Option<String>,
    pub actual_state: Option<String>,
    pub resolved: bool,
    pub detected_at: String,
}
