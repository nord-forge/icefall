use sqlx::{Row, SqlitePool};

use crate::db::DbError;

pub(super) async fn get_deploy_analytics(
    pool: &SqlitePool,
    from: &str,
    to: &str,
) -> Result<serde_json::Value, DbError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deploys WHERE created_at >= ? AND created_at <= ?",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    let successful: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deploys WHERE status = 'running' AND created_at >= ? AND created_at <= ?",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    let failed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deploys WHERE status = 'failed' AND created_at >= ? AND created_at <= ?",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    let avg_build_secs: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(CAST((julianday(finished_at) - julianday(started_at)) * 86400 AS REAL))
         FROM deploys WHERE status = 'running' AND started_at IS NOT NULL AND finished_at IS NOT NULL
         AND created_at >= ? AND created_at <= ?",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    let per_app = sqlx::query(
        "SELECT app_id, COUNT(*) as deploy_count,
                SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as fail_count
         FROM deploys WHERE created_at >= ? AND created_at <= ?
         GROUP BY app_id ORDER BY deploy_count DESC LIMIT 20",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    let per_app_data: Vec<serde_json::Value> = per_app
        .iter()
        .map(|r| {
            serde_json::json!({
                "app_id": r.get::<String, _>("app_id"),
                "deploy_count": r.get::<i64, _>("deploy_count"),
                "success_count": r.get::<i64, _>("success_count"),
                "fail_count": r.get::<i64, _>("fail_count"),
            })
        })
        .collect();

    let success_rate = if total > 0 {
        (successful as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };

    Ok(serde_json::json!({
        "total_deploys": total,
        "successful": successful,
        "failed": failed,
        "success_rate": success_rate,
        "avg_build_time_secs": avg_build_secs.unwrap_or(0.0).round(),
        "per_app": per_app_data,
    }))
}
