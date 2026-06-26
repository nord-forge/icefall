use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait NotificationStore: Send + Sync {
    // Notifications
    async fn create_notification_channel(
        &self,
        channel: &NewNotification,
    ) -> Result<Notification, DbError>;
    async fn list_notification_channels(&self) -> Result<Vec<Notification>, DbError>;
    /// Permanently delete a channel and (via FK cascade) its subscription rules.
    async fn delete_notification_channel(&self, id: &str) -> Result<(), DbError>;
    async fn create_notification_rule(
        &self,
        rule: &NewNotificationRule,
    ) -> Result<NotificationRule, DbError>;
    async fn get_notification_rules(&self, app_id: &str) -> Result<Vec<NotificationRule>, DbError>;
    /// All rules subscribed to `event_type`, across scopes (IF-167 dispatch).
    async fn get_notification_rules_by_event(
        &self,
        event_type: &str,
    ) -> Result<Vec<NotificationRule>, DbError>;
}
