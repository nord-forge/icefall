use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_notification_channel(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    channel: &NewNotification,
) -> Result<Notification, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let encrypted_config = encryptor.encrypt(channel.config.as_bytes())?;

    sqlx::query(
        "INSERT INTO notifications (id, channel_type, config_encrypted, created_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&channel.channel_type)
    .bind(&encrypted_config)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(Notification {
        id,
        channel_type: channel.channel_type.clone(),
        config: channel.config.clone(),
        created_at: now,
    })
}

pub(super) async fn list_notification_channels(
    pool: &SqlitePool,
    encryptor: &Encryptor,
) -> Result<Vec<Notification>, DbError> {
    let rows = sqlx::query(
        "SELECT id, channel_type, config_encrypted, created_at FROM notifications ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    let mut channels = Vec::with_capacity(rows.len());
    for row in rows {
        let encrypted: Vec<u8> = row.get("config_encrypted");
        let decrypted = encryptor.decrypt(&encrypted)?;
        let config = String::from_utf8(decrypted).unwrap_or_default();

        channels.push(Notification {
            id: row.get("id"),
            channel_type: row.get("channel_type"),
            config,
            created_at: row.get("created_at"),
        });
    }
    Ok(channels)
}

pub(super) async fn create_notification_rule(
    pool: &SqlitePool,
    rule: &NewNotificationRule,
) -> Result<NotificationRule, DbError> {
    let id = new_id();

    sqlx::query(
        "INSERT INTO notification_rules (id, app_id, notification_id, event_type)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&rule.app_id)
    .bind(&rule.notification_id)
    .bind(&rule.event_type)
    .execute(pool)
    .await?;

    Ok(NotificationRule {
        id,
        app_id: rule.app_id.clone(),
        notification_id: rule.notification_id.clone(),
        event_type: rule.event_type.clone(),
    })
}

pub(super) async fn get_notification_rules(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Vec<NotificationRule>, DbError> {
    let rules = sqlx::query_as::<_, NotificationRule>(
        "SELECT * FROM notification_rules WHERE app_id = ? ORDER BY event_type",
    )
    .bind(app_id)
    .fetch_all(pool)
    .await?;
    Ok(rules)
}
