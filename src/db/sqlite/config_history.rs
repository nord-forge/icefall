use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn record_config_change(
    pool: &SqlitePool,
    resource_type: &str,
    resource_id: &str,
    field: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    changed_by: Option<&str>,
) -> Result<(), DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO config_history (id, resource_type, resource_id, field, old_value, new_value, changed_by, changed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(field)
    .bind(old_value)
    .bind(new_value)
    .bind(changed_by)
    .bind(&now)
    .execute(pool)
    .await?;

    // Prune old entries (keep last 100 per resource)
    sqlx::query(
        "DELETE FROM config_history WHERE id IN (
            SELECT id FROM config_history
            WHERE resource_type = ? AND resource_id = ?
            ORDER BY changed_at DESC
            LIMIT -1 OFFSET 100
        )",
    )
    .bind(resource_type)
    .bind(resource_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(super) async fn list_config_history(
    pool: &SqlitePool,
    resource_type: &str,
    resource_id: &str,
    limit: i64,
) -> Result<Vec<ConfigHistoryEntry>, DbError> {
    let entries = sqlx::query_as::<_, ConfigHistoryEntry>(
        "SELECT * FROM config_history WHERE resource_type = ? AND resource_id = ? ORDER BY changed_at DESC LIMIT ?",
    )
    .bind(resource_type)
    .bind(resource_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(entries)
}
