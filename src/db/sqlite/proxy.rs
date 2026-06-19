use sqlx::SqlitePool;

use crate::db::models::{
    new_id, now_iso8601, ProxyConfigHistory, ProxySettings, UpdateProxySettings,
};
use crate::db::DbError;

/// Number of historical proxy configs retained per app for rollback.
const HISTORY_LIMIT: i64 = 10;

/// Snapshot the given config for an app, then prune so at most `HISTORY_LIMIT`
/// rows remain (oldest deleted first). Called before every proxy change.
pub(super) async fn record_proxy_config_history(
    pool: &SqlitePool,
    app_id: &str,
    config: &str,
) -> Result<(), DbError> {
    let id = new_id();
    let now = now_iso8601();
    sqlx::query(
        "INSERT INTO proxy_config_history (id, app_id, config, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(app_id)
    .bind(config)
    .bind(&now)
    .execute(pool)
    .await?;

    // Keep only the newest HISTORY_LIMIT rows for this app, ordered by rowid
    // (true insertion order) since millisecond timestamps can tie.
    sqlx::query(
        "DELETE FROM proxy_config_history
         WHERE app_id = ?
           AND id NOT IN (
               SELECT id FROM proxy_config_history
               WHERE app_id = ?
               ORDER BY rowid DESC
               LIMIT ?
           )",
    )
    .bind(app_id)
    .bind(app_id)
    .bind(HISTORY_LIMIT)
    .execute(pool)
    .await?;

    Ok(())
}

pub(super) async fn list_proxy_config_history(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Vec<ProxyConfigHistory>, DbError> {
    let rows = sqlx::query_as::<_, ProxyConfigHistory>(
        "SELECT id, app_id, config, created_at FROM proxy_config_history
         WHERE app_id = ? ORDER BY rowid DESC LIMIT ?",
    )
    .bind(app_id)
    .bind(HISTORY_LIMIT)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Most recent history snapshot for an app, used by "Undo Last Change".
pub(super) async fn latest_proxy_config_history(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Option<ProxyConfigHistory>, DbError> {
    let row = sqlx::query_as::<_, ProxyConfigHistory>(
        "SELECT id, app_id, config, created_at FROM proxy_config_history
         WHERE app_id = ? ORDER BY rowid DESC LIMIT 1",
    )
    .bind(app_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Store the JSON-encoded preset configuration for an app. Presets coexist with
/// auto-generation, so this does NOT set has_custom_proxy_config.
pub(super) async fn set_proxy_presets(
    pool: &SqlitePool,
    app_id: &str,
    presets: &str,
) -> Result<(), DbError> {
    let res = sqlx::query("UPDATE apps SET proxy_presets = ?, updated_at = ? WHERE id = ?")
        .bind(presets)
        .bind(now_iso8601())
        .bind(app_id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(DbError::NotFound(app_id.to_string()));
    }
    Ok(())
}

/// Save a raw custom config and flip the app into advanced mode. While in
/// advanced mode Icefall does not regenerate the app's proxy config on deploy.
pub(super) async fn set_custom_proxy_config(
    pool: &SqlitePool,
    app_id: &str,
    config: &str,
) -> Result<(), DbError> {
    let res = sqlx::query(
        "UPDATE apps SET custom_proxy_config = ?, has_custom_proxy_config = TRUE, updated_at = ?
         WHERE id = ?",
    )
    .bind(config)
    .bind(now_iso8601())
    .bind(app_id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(DbError::NotFound(app_id.to_string()));
    }
    Ok(())
}

/// Discard any custom config and return the app to preset/auto-generated mode.
pub(super) async fn clear_custom_proxy_config(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<(), DbError> {
    let res = sqlx::query(
        "UPDATE apps SET custom_proxy_config = NULL, has_custom_proxy_config = FALSE, updated_at = ?
         WHERE id = ?",
    )
    .bind(now_iso8601())
    .bind(app_id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(DbError::NotFound(app_id.to_string()));
    }
    Ok(())
}

pub(super) async fn get_proxy_settings(pool: &SqlitePool) -> Result<ProxySettings, DbError> {
    // Row is seeded by the migration; fall back to defaults if it was deleted.
    let row = sqlx::query_as::<_, ProxySettings>(
        "SELECT id, default_headers, default_rate_limit, force_https,
                public_port_range_start, public_port_range_end, updated_at
         FROM proxy_settings WHERE id = 'global'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.unwrap_or_else(|| ProxySettings {
        id: "global".to_string(),
        default_headers: None,
        default_rate_limit: None,
        force_https: true,
        public_port_range_start: 10000,
        public_port_range_end: 10100,
        updated_at: now_iso8601(),
    }))
}

pub(super) async fn update_proxy_settings(
    pool: &SqlitePool,
    update: &UpdateProxySettings,
) -> Result<ProxySettings, DbError> {
    let existing = get_proxy_settings(pool).await?;
    let default_headers = match &update.default_headers {
        Some(v) => v.as_deref(),
        None => existing.default_headers.as_deref(),
    };
    let default_rate_limit = match &update.default_rate_limit {
        Some(v) => v.as_deref(),
        None => existing.default_rate_limit.as_deref(),
    };
    let force_https = update.force_https.unwrap_or(existing.force_https);
    let range_start = update
        .public_port_range_start
        .unwrap_or(existing.public_port_range_start);
    let range_end = update
        .public_port_range_end
        .unwrap_or(existing.public_port_range_end);
    let now = now_iso8601();

    // Upsert — the global row normally exists, but re-create it if it was removed.
    sqlx::query(
        "INSERT INTO proxy_settings (id, default_headers, default_rate_limit, force_https,
                                     public_port_range_start, public_port_range_end, updated_at)
         VALUES ('global', ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            default_headers = excluded.default_headers,
            default_rate_limit = excluded.default_rate_limit,
            force_https = excluded.force_https,
            public_port_range_start = excluded.public_port_range_start,
            public_port_range_end = excluded.public_port_range_end,
            updated_at = excluded.updated_at",
    )
    .bind(default_headers)
    .bind(default_rate_limit)
    .bind(force_https)
    .bind(range_start)
    .bind(range_end)
    .bind(&now)
    .execute(pool)
    .await?;

    get_proxy_settings(pool).await
}
