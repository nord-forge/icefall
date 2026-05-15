use sqlx::{Row, SqlitePool};

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_log_drain(
    pool: &SqlitePool,
    drain: &NewLogDrain,
) -> Result<LogDrain, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO log_drains (id, app_id, name, drain_type, config, enabled, error_count, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 1, 0, ?, ?)",
    )
    .bind(&id)
    .bind(&drain.app_id)
    .bind(&drain.name)
    .bind(&drain.drain_type)
    .bind(&drain.config)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(LogDrain {
        id,
        app_id: drain.app_id.clone(),
        name: drain.name.clone(),
        drain_type: drain.drain_type.clone(),
        config: drain.config.clone(),
        enabled: true,
        last_sent_at: None,
        error_count: 0,
        last_error: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(super) async fn list_log_drains_for_app(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Vec<LogDrain>, DbError> {
    let rows = sqlx::query(
        "SELECT id, app_id, name, drain_type, config, enabled, last_sent_at, error_count, last_error, created_at, updated_at
         FROM log_drains WHERE app_id = ? ORDER BY created_at DESC",
    )
    .bind(app_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_log_drain).collect())
}

pub(super) async fn list_global_log_drains(pool: &SqlitePool) -> Result<Vec<LogDrain>, DbError> {
    let rows = sqlx::query(
        "SELECT id, app_id, name, drain_type, config, enabled, last_sent_at, error_count, last_error, created_at, updated_at
         FROM log_drains WHERE app_id IS NULL ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_log_drain).collect())
}

pub(super) async fn update_log_drain(
    pool: &SqlitePool,
    id: &str,
    drain: &NewLogDrain,
) -> Result<LogDrain, DbError> {
    let now = now_iso8601();

    sqlx::query(
        "UPDATE log_drains SET name = ?, drain_type = ?, config = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&drain.name)
    .bind(&drain.drain_type)
    .bind(&drain.config)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_log_drain(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("log_drain {id}")))
}

pub(super) async fn delete_log_drain(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM log_drains WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn get_log_drain(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<LogDrain>, DbError> {
    let row = sqlx::query(
        "SELECT id, app_id, name, drain_type, config, enabled, last_sent_at, error_count, last_error, created_at, updated_at
         FROM log_drains WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(row_to_log_drain))
}

fn row_to_log_drain(r: sqlx::sqlite::SqliteRow) -> LogDrain {
    LogDrain {
        id: r.get("id"),
        app_id: r.get("app_id"),
        name: r.get("name"),
        drain_type: r.get("drain_type"),
        config: r.get("config"),
        enabled: r.get("enabled"),
        last_sent_at: r.get("last_sent_at"),
        error_count: r.get("error_count"),
        last_error: r.get("last_error"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}
