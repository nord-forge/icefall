use sqlx::SqlitePool;

use crate::db::models::{new_id, now_iso8601, ContainerUsageStats, NewContainerMetricsRecord};
use crate::db::DbError;

pub(super) async fn record_container_metrics(
    pool: &SqlitePool,
    record: &NewContainerMetricsRecord,
) -> Result<(), DbError> {
    let id = new_id();
    let now = now_iso8601();
    sqlx::query(
        "INSERT INTO container_metrics_history
            (id, app_id, cpu_percent, memory_usage_bytes, memory_limit_bytes, recorded_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&record.app_id)
    .bind(record.cpu_percent)
    .bind(record.memory_usage_bytes)
    .bind(record.memory_limit_bytes)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Aggregated usage per app over the last `days`, for the recommendations
/// engine. `memory_limit_bytes` comes from the most recent sample.
pub(super) async fn container_usage_stats(
    pool: &SqlitePool,
    days: i64,
) -> Result<Vec<ContainerUsageStats>, DbError> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();

    // A correlated subquery pulls the latest limit per app; the outer aggregate
    // computes avg/peak. Cheap at our row counts and avoids a second round-trip.
    let rows = sqlx::query_as::<_, ContainerUsageStats>(
        "SELECT
            app_id,
            AVG(cpu_percent)               AS avg_cpu_percent,
            MAX(cpu_percent)               AS peak_cpu_percent,
            CAST(AVG(memory_usage_bytes) AS INTEGER) AS avg_memory_bytes,
            MAX(memory_usage_bytes)        AS peak_memory_bytes,
            (SELECT memory_limit_bytes FROM container_metrics_history m2
              WHERE m2.app_id = m1.app_id ORDER BY recorded_at DESC LIMIT 1) AS memory_limit_bytes,
            COUNT(*)                       AS sample_count
         FROM container_metrics_history m1
         WHERE recorded_at >= ?
         GROUP BY app_id",
    )
    .bind(&cutoff)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Delete samples older than `keep_days`. Called periodically so the table
/// doesn't grow without bound; the analysis window is shorter than retention.
pub(super) async fn prune_container_metrics(
    pool: &SqlitePool,
    keep_days: i64,
) -> Result<u64, DbError> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(keep_days)).to_rfc3339();
    let result = sqlx::query("DELETE FROM container_metrics_history WHERE recorded_at < ?")
        .bind(&cutoff)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
