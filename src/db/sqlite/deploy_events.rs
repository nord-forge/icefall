use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn record_deploy_event(
    pool: &SqlitePool,
    deploy_id: &str,
    event_type: &str,
    data: &serde_json::Value,
) -> Result<(), DbError> {
    let id = new_id();
    let now = now_iso8601();
    let data_str = serde_json::to_string(data).unwrap_or_default();

    sqlx::query(
        "INSERT INTO deploy_events (id, deploy_id, event_type, data, timestamp)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(deploy_id)
    .bind(event_type)
    .bind(&data_str)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn list_deploy_events(
    pool: &SqlitePool,
    deploy_id: &str,
) -> Result<Vec<DeployEvent>, DbError> {
    let events = sqlx::query_as::<_, DeployEvent>(
        "SELECT * FROM deploy_events WHERE deploy_id = ? ORDER BY timestamp ASC",
    )
    .bind(deploy_id)
    .fetch_all(pool)
    .await?;
    Ok(events)
}
