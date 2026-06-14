use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScheduledTask {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub command: String,
    pub cron_expression: String,
    pub timeout_seconds: i32,
    pub enabled: bool,
    pub container_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewScheduledTask {
    pub app_id: String,
    pub name: String,
    pub command: String,
    pub cron_expression: String,
    pub timeout_seconds: Option<i32>,
    pub container_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScheduledTaskExecution {
    pub id: String,
    pub task_id: String,
    pub status: String,
    pub output: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}
