use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

// --- Update State ---

pub(super) async fn get_update_state(pool: &SqlitePool) -> Result<UpdateState, DbError> {
    let state = sqlx::query_as::<_, UpdateState>("SELECT * FROM update_state WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok(state)
}

pub(super) async fn set_update_available(
    pool: &SqlitePool,
    version: &str,
    release_url: &str,
    release_notes: &str,
    highlights: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE update_state SET available_version = ?, release_url = ?, release_notes = ?, changelog_highlights = ?, download_state = 'none', download_progress = 0, download_path = NULL, error_message = NULL WHERE id = 1",
    )
    .bind(version)
    .bind(release_url)
    .bind(release_notes)
    .bind(highlights)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn set_update_download_state(
    pool: &SqlitePool,
    state: &str,
    progress: i64,
    path: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE update_state SET download_state = ?, download_progress = ?, download_path = ? WHERE id = 1",
    )
    .bind(state)
    .bind(progress)
    .bind(path)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn set_update_error(pool: &SqlitePool, error: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE update_state SET error_message = ?, download_state = 'error' WHERE id = 1")
        .bind(error)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn clear_update_available(pool: &SqlitePool) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE update_state SET available_version = NULL, release_url = NULL, release_notes = NULL, changelog_highlights = NULL, download_state = 'none', download_progress = 0, download_path = NULL, error_message = NULL WHERE id = 1",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn update_highest_seen(pool: &SqlitePool, version: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE update_state SET highest_seen_version = ? WHERE id = 1")
        .bind(version)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn set_last_check_at(pool: &SqlitePool, timestamp: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE update_state SET last_check_at = ? WHERE id = 1")
        .bind(timestamp)
        .execute(pool)
        .await?;
    Ok(())
}

// --- Update Preferences ---

pub(super) async fn set_update_channel(pool: &SqlitePool, channel: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE update_state SET channel = ? WHERE id = 1")
        .bind(channel)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn set_auto_update_settings(
    pool: &SqlitePool,
    enabled: bool,
    channel: &str,
    window_start: &str,
    window_end: &str,
    notify_before_minutes: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE update_state SET auto_update_enabled = ?, auto_update_channel = ?, auto_update_window_start = ?, auto_update_window_end = ?, auto_update_notify_before_minutes = ? WHERE id = 1",
    )
    .bind(enabled)
    .bind(channel)
    .bind(window_start)
    .bind(window_end)
    .bind(notify_before_minutes)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn set_auto_update_pre_downloaded(
    pool: &SqlitePool,
    pre_downloaded: bool,
) -> Result<(), DbError> {
    sqlx::query("UPDATE update_state SET auto_update_pre_downloaded = ? WHERE id = 1")
        .bind(pre_downloaded)
        .execute(pool)
        .await?;
    Ok(())
}

// --- Skipped Versions ---

pub(super) async fn skip_update_version(pool: &SqlitePool, version: &str) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query("INSERT OR REPLACE INTO skipped_updates (version, skipped_at) VALUES (?, ?)")
        .bind(version)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn is_version_skipped(pool: &SqlitePool, version: &str) -> Result<bool, DbError> {
    let row = sqlx::query("SELECT version FROM skipped_updates WHERE version = ?")
        .bind(version)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

// --- Update History ---

pub(super) async fn record_update_history(
    pool: &SqlitePool,
    entry: &UpdateHistoryEntry,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO update_history (id, version, previous_version, status, duration_secs, error, changelog_url, applied_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry.id)
    .bind(&entry.version)
    .bind(&entry.previous_version)
    .bind(&entry.status)
    .bind(entry.duration_secs)
    .bind(&entry.error)
    .bind(&entry.changelog_url)
    .bind(&entry.applied_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn list_update_history(
    pool: &SqlitePool,
    limit: usize,
) -> Result<Vec<UpdateHistoryEntry>, DbError> {
    let entries = sqlx::query_as::<_, UpdateHistoryEntry>(
        "SELECT * FROM update_history ORDER BY applied_at DESC LIMIT ?",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;
    Ok(entries)
}
