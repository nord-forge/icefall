use sqlx::{Row, SqlitePool};

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn get_instance_backup_config(
    pool: &SqlitePool,
) -> Result<Option<InstanceBackupConfig>, DbError> {
    let config = sqlx::query_as::<_, InstanceBackupConfig>(
        "SELECT * FROM instance_backup_config WHERE id = 'singleton'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(config)
}

pub(super) async fn upsert_instance_backup_config(
    pool: &SqlitePool,
    enabled: bool,
    cron_schedule: &str,
    retention_count: i64,
) -> Result<InstanceBackupConfig, DbError> {
    let now = now_iso8601();
    sqlx::query(
        "INSERT INTO instance_backup_config (id, enabled, cron_schedule, retention_count, updated_at)
         VALUES ('singleton', ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET enabled = excluded.enabled, cron_schedule = excluded.cron_schedule, retention_count = excluded.retention_count, updated_at = excluded.updated_at",
    )
    .bind(enabled)
    .bind(cron_schedule)
    .bind(retention_count)
    .bind(&now)
    .execute(pool)
    .await?;

    get_instance_backup_config(pool)
        .await?
        .ok_or_else(|| DbError::NotFound("instance_backup_config".to_string()))
}

pub(super) async fn create_instance_backup_record(
    pool: &SqlitePool,
    filename: &str,
    s3_key: Option<&str>,
) -> Result<InstanceBackupRecord, DbError> {
    let id = new_id();
    let now = now_iso8601();
    sqlx::query(
        "INSERT INTO instance_backup_history (id, filename, size_bytes, status, s3_key, started_at)
         VALUES (?, ?, 0, 'running', ?, ?)",
    )
    .bind(&id)
    .bind(filename)
    .bind(s3_key)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(InstanceBackupRecord {
        id,
        filename: filename.to_string(),
        size_bytes: 0,
        status: "running".to_string(),
        error_message: None,
        s3_key: s3_key.map(String::from),
        started_at: now,
        finished_at: None,
    })
}

/// Mark every still-"running" instance backup as failed. Called at startup:
/// a running record means the process that owned it died mid-backup (deploy,
/// restart, crash), so it can never complete and would otherwise show
/// "In progress" forever.
pub(super) async fn fail_stale_instance_backups(pool: &SqlitePool) -> Result<u64, DbError> {
    let now = now_iso8601();
    let result = sqlx::query(
        "UPDATE instance_backup_history
         SET status = 'failed',
             error_message = 'Interrupted — the server restarted while this backup was running',
             finished_at = ?
         WHERE status = 'running'",
    )
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub(super) async fn has_running_instance_backup(pool: &SqlitePool) -> Result<bool, DbError> {
    let row = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM instance_backup_history WHERE status = 'running') AS running",
    )
    .fetch_one(pool)
    .await?;
    let running: i64 = row.get("running");
    Ok(running != 0)
}

pub(super) async fn update_instance_backup_record(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    size_bytes: i64,
    error_message: Option<&str>,
) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query(
        "UPDATE instance_backup_history SET status = ?, size_bytes = ?, error_message = ?, finished_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(size_bytes)
    .bind(error_message)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn list_instance_backup_history(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<InstanceBackupRecord>, DbError> {
    let records = sqlx::query_as::<_, InstanceBackupRecord>(
        "SELECT * FROM instance_backup_history ORDER BY started_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub(super) async fn delete_instance_backup_record(
    pool: &SqlitePool,
    id: &str,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM instance_backup_history WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
