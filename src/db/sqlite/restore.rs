use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_restore_record(
    pool: &SqlitePool,
    database_id: &str,
    source_type: &str,
    source_ref: Option<&str>,
) -> Result<DatabaseRestoreRecord, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO database_restore_history (id, database_id, source_type, source_ref, status, started_at)
         VALUES (?, ?, ?, ?, 'running', ?)",
    )
    .bind(&id)
    .bind(database_id)
    .bind(source_type)
    .bind(source_ref)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, DatabaseRestoreRecord>(
        "SELECT * FROM database_restore_history WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await
    .map_err(DbError::from)
}

pub(super) async fn update_restore_record(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    output: Option<&str>,
) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query(
        "UPDATE database_restore_history SET status = ?, output = ?, finished_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(output)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn list_restore_history(
    pool: &SqlitePool,
    database_id: &str,
    limit: i64,
) -> Result<Vec<DatabaseRestoreRecord>, DbError> {
    let records = sqlx::query_as::<_, DatabaseRestoreRecord>(
        "SELECT * FROM database_restore_history WHERE database_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(database_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(records)
}
