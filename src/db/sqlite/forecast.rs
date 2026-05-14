use sqlx::{Row, SqlitePool};

use crate::db::DbError;

/// Returns (disk_used_ratio, memory_used_ratio, cpu_percent) daily averages for forecasting
pub(super) async fn get_server_metrics_for_forecast(
    pool: &SqlitePool,
    server_id: &str,
    days: i64,
) -> Result<Vec<(f64, f64, f64)>, DbError> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();

    let rows = sqlx::query(
        "SELECT
            AVG(CAST(disk_used_bytes AS REAL) / NULLIF(disk_total_bytes, 0)) as disk_ratio,
            AVG(CAST(ram_used_bytes AS REAL) / NULLIF(ram_total_bytes, 0)) as mem_ratio,
            AVG(cpu_percent) as cpu_avg
         FROM server_metrics_history
         WHERE server_id = ? AND recorded_at >= ?
         GROUP BY DATE(recorded_at)
         ORDER BY DATE(recorded_at) ASC",
    )
    .bind(server_id)
    .bind(&cutoff)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let disk: f64 = r.try_get("disk_ratio").unwrap_or(0.0);
            let mem: f64 = r.try_get("mem_ratio").unwrap_or(0.0);
            let cpu: f64 = r.try_get("cpu_avg").unwrap_or(0.0);
            (disk, mem, cpu)
        })
        .collect())
}
