use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn list_scheduled_tasks(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Vec<ScheduledTask>, DbError> {
    let tasks = sqlx::query_as::<_, ScheduledTask>(
        "SELECT * FROM scheduled_tasks WHERE app_id = ? ORDER BY created_at",
    )
    .bind(app_id)
    .fetch_all(pool)
    .await?;
    Ok(tasks)
}

pub(super) async fn create_scheduled_task(
    pool: &SqlitePool,
    task: &NewScheduledTask,
) -> Result<ScheduledTask, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let timeout = task.timeout_seconds.unwrap_or(300);

    sqlx::query(
        "INSERT INTO scheduled_tasks (id, app_id, name, command, cron_expression, timeout_seconds, enabled, container_name, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, TRUE, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&task.app_id)
    .bind(&task.name)
    .bind(&task.command)
    .bind(&task.cron_expression)
    .bind(timeout)
    .bind(&task.container_name)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, ScheduledTask>("SELECT * FROM scheduled_tasks WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(DbError::from)
}

pub(super) async fn update_scheduled_task_enabled(
    pool: &SqlitePool,
    id: &str,
    enabled: bool,
) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query("UPDATE scheduled_tasks SET enabled = ?, updated_at = ? WHERE id = ?")
        .bind(enabled)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn delete_scheduled_task(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM scheduled_tasks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn list_all_enabled_scheduled_tasks(
    pool: &SqlitePool,
) -> Result<Vec<ScheduledTask>, DbError> {
    let tasks = sqlx::query_as::<_, ScheduledTask>(
        "SELECT * FROM scheduled_tasks WHERE enabled = TRUE",
    )
    .fetch_all(pool)
    .await?;
    Ok(tasks)
}

pub(super) async fn create_task_execution(
    pool: &SqlitePool,
    task_id: &str,
    status: &str,
    output: Option<&str>,
) -> Result<ScheduledTaskExecution, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO scheduled_task_executions (id, task_id, status, output, started_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(task_id)
    .bind(status)
    .bind(output)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, ScheduledTaskExecution>(
        "SELECT * FROM scheduled_task_executions WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await
    .map_err(DbError::from)
}

pub(super) async fn update_task_execution(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    output: Option<&str>,
) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query(
        "UPDATE scheduled_task_executions SET status = ?, output = COALESCE(?, output), finished_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(output)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn list_task_executions(
    pool: &SqlitePool,
    task_id: &str,
    limit: i64,
) -> Result<Vec<ScheduledTaskExecution>, DbError> {
    let execs = sqlx::query_as::<_, ScheduledTaskExecution>(
        "SELECT * FROM scheduled_task_executions WHERE task_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(task_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(execs)
}
