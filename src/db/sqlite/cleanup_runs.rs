use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_cleanup_run(pool: &SqlitePool) -> Result<CleanupRun, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query("INSERT INTO cleanup_runs (id, started_at, status) VALUES (?, ?, 'running')")
        .bind(&id)
        .bind(&now)
        .execute(pool)
        .await?;

    Ok(CleanupRun {
        id,
        started_at: now,
        finished_at: None,
        status: "running".to_string(),
        freed_bytes: 0,
        removed_items: 0,
        error: None,
        details: None,
    })
}

pub(super) async fn finish_cleanup_run(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    freed_bytes: i64,
    removed_items: i64,
    error: Option<&str>,
    details: Option<&str>,
) -> Result<(), DbError> {
    let now = now_iso8601();

    sqlx::query(
        "UPDATE cleanup_runs SET finished_at = ?, status = ?, freed_bytes = ?, removed_items = ?, error = ?, details = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(status)
    .bind(freed_bytes)
    .bind(removed_items)
    .bind(error)
    .bind(details)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(super) async fn list_cleanup_runs(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<CleanupRun>, DbError> {
    let runs = sqlx::query_as::<_, CleanupRun>(
        "SELECT * FROM cleanup_runs ORDER BY started_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(runs)
}
