use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

#[allow(clippy::too_many_arguments)]
pub(super) async fn store_canary_result(
    pool: &SqlitePool,
    deploy_id: &str,
    p50: f64,
    p95: f64,
    p99: f64,
    errors: i32,
    total: i32,
    verdict: &str,
) -> Result<CanaryResult, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO canary_results (id, deploy_id, p50_ms, p95_ms, p99_ms, error_count, total_requests, verdict, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(deploy_id)
    .bind(p50)
    .bind(p95)
    .bind(p99)
    .bind(errors)
    .bind(total)
    .bind(verdict)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(CanaryResult {
        id,
        deploy_id: deploy_id.to_string(),
        p50_ms: Some(p50),
        p95_ms: Some(p95),
        p99_ms: Some(p99),
        error_count: errors,
        total_requests: total,
        verdict: verdict.to_string(),
        created_at: now,
    })
}

pub(super) async fn get_canary_baseline(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Option<CanaryResult>, DbError> {
    let result = sqlx::query_as::<_, CanaryResult>(
        "SELECT cr.* FROM canary_results cr
         JOIN deploys d ON cr.deploy_id = d.id
         WHERE d.app_id = ? AND cr.verdict IN ('pass', 'baseline')
         ORDER BY cr.created_at DESC LIMIT 1",
    )
    .bind(app_id)
    .fetch_optional(pool)
    .await?;
    Ok(result)
}
