use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_cleanup_execution(
    pool: &SqlitePool,
    server_id: &str,
) -> Result<ContainerCleanupExecution, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO container_cleanup_executions (id, server_id, started_at, status)
         VALUES (?, ?, ?, 'running')",
    )
    .bind(&id)
    .bind(server_id)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, ContainerCleanupExecution>(
        "SELECT * FROM container_cleanup_executions WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await
    .map_err(DbError::from)
}

pub(super) async fn update_cleanup_execution(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    space_reclaimed: Option<i64>,
    images: i32,
    volumes: i32,
    networks: i32,
) -> Result<(), DbError> {
    let now = now_iso8601();

    sqlx::query(
        "UPDATE container_cleanup_executions
         SET status = ?, finished_at = ?, space_reclaimed_bytes = ?,
             images_removed = ?, volumes_removed = ?, networks_removed = ?
         WHERE id = ?",
    )
    .bind(status)
    .bind(&now)
    .bind(space_reclaimed)
    .bind(images)
    .bind(volumes)
    .bind(networks)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(super) async fn list_cleanup_executions(
    pool: &SqlitePool,
    server_id: &str,
    limit: i64,
) -> Result<Vec<ContainerCleanupExecution>, DbError> {
    let execs = sqlx::query_as::<_, ContainerCleanupExecution>(
        "SELECT * FROM container_cleanup_executions WHERE server_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(server_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(execs)
}
