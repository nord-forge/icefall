use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDrain {
    pub id: String,
    pub app_id: Option<String>,
    pub name: String,
    pub drain_type: String,
    pub config: String,
    pub enabled: bool,
    pub last_sent_at: Option<String>,
    pub error_count: i32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewLogDrain {
    pub app_id: Option<String>,
    pub name: String,
    pub drain_type: String,
    pub config: String,
}
