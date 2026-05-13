use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub channel_type: String,
    pub config: String,
    pub created_at: String,
}

pub struct NewNotification {
    pub channel_type: String,
    pub config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationRule {
    pub id: String,
    pub app_id: String,
    pub notification_id: String,
    pub event_type: String,
}

pub struct NewNotificationRule {
    pub app_id: String,
    pub notification_id: String,
    pub event_type: String,
}
