use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

// --- Cleanup / Pruning ---

pub(super) async fn prune_expired_sessions(
    pool: &SqlitePool,
    older_than: &str,
) -> Result<u64, DbError> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
        .bind(older_than)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub(super) async fn prune_expired_tokens(pool: &SqlitePool) -> Result<u64, DbError> {
    let now = now_iso8601();
    let result =
        sqlx::query("DELETE FROM api_tokens WHERE expires_at IS NOT NULL AND expires_at < ?")
            .bind(&now)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

pub(super) async fn prune_expired_invitations(pool: &SqlitePool) -> Result<u64, DbError> {
    let now = now_iso8601();
    let result = sqlx::query("DELETE FROM invitations WHERE expires_at < ?")
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub(super) async fn prune_health_check_events(
    pool: &SqlitePool,
    older_than: &str,
) -> Result<u64, DbError> {
    let result = sqlx::query("DELETE FROM health_check_events WHERE checked_at < ?")
        .bind(older_than)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub(super) async fn prune_old_deploys(
    pool: &SqlitePool,
    older_than: &str,
    keep_per_app: i64,
) -> Result<u64, DbError> {
    let result = sqlx::query(
        "DELETE FROM deploys WHERE created_at < ? AND id NOT IN (
            SELECT id FROM deploys d2
            WHERE d2.app_id = deploys.app_id
            ORDER BY d2.created_at DESC
            LIMIT ?
        )",
    )
    .bind(older_than)
    .bind(keep_per_app)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub(super) async fn prune_server_metrics(
    pool: &SqlitePool,
    older_than: &str,
) -> Result<u64, DbError> {
    let result = sqlx::query("DELETE FROM server_metrics WHERE timestamp < ?")
        .bind(older_than)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub(super) async fn prune_server_metrics_history(
    pool: &SqlitePool,
    older_than: &str,
) -> Result<u64, DbError> {
    let result = sqlx::query("DELETE FROM server_metrics_history WHERE recorded_at < ?")
        .bind(older_than)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// --- Backup (VACUUM) ---

pub(super) async fn vacuum_into(pool: &SqlitePool, path: &str) -> Result<(), DbError> {
    // VACUUM INTO interpolates a path into SQL. The target file must not yet
    // exist, so canonicalize its parent dir and require the resolved path to
    // stay inside it — defence in depth if `path` ever takes user input.
    let target = std::path::Path::new(path);
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| DbError::InvalidInput("VACUUM path has no parent directory".into()))?;
    let file_name = target
        .file_name()
        .ok_or_else(|| DbError::InvalidInput("VACUUM path has no file name".into()))?;
    let canonical_parent = std::fs::canonicalize(parent)
        .map_err(|e| DbError::InvalidInput(format!("VACUUM parent directory unresolved: {e}")))?;
    let safe_path = canonical_parent.join(file_name);
    if !safe_path.starts_with(&canonical_parent) {
        return Err(DbError::InvalidInput(
            "VACUUM path escapes its directory".into(),
        ));
    }

    let safe_str = safe_path.to_string_lossy();
    sqlx::query(&format!("VACUUM INTO '{}'", safe_str.replace('\'', "''")))
        .execute(pool)
        .await?;
    Ok(())
}

// --- Migrations ---

pub(super) async fn run_migrations(pool: &SqlitePool) -> Result<(), DbError> {
    sqlx::migrate!("src/db/migrations").run(pool).await?;
    Ok(())
}
