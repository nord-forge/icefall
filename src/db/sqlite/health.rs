use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_health_check(
    pool: &SqlitePool,
    hc: &NewHealthCheck,
) -> Result<HealthCheck, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO health_checks (id, app_id, check_type, config, interval_secs, failure_threshold, auto_restart, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&hc.app_id)
    .bind(&hc.check_type)
    .bind(&hc.config)
    .bind(hc.interval_secs)
    .bind(hc.failure_threshold)
    .bind(hc.auto_restart)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(HealthCheck {
        id,
        app_id: hc.app_id.clone(),
        check_type: hc.check_type.clone(),
        config: hc.config.clone(),
        interval_secs: hc.interval_secs,
        failure_threshold: hc.failure_threshold,
        auto_restart: hc.auto_restart,
        created_at: now,
    })
}

pub(super) async fn get_health_checks(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Vec<HealthCheck>, DbError> {
    let checks = sqlx::query_as::<_, HealthCheck>(
        "SELECT * FROM health_checks WHERE app_id = ? ORDER BY created_at",
    )
    .bind(app_id)
    .fetch_all(pool)
    .await?;
    Ok(checks)
}

pub(super) async fn record_health_event(
    pool: &SqlitePool,
    event: &NewHealthCheckEvent,
) -> Result<(), DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO health_check_events (id, health_check_id, status, checked_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&event.health_check_id)
    .bind(&event.status)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

pub(super) async fn get_health_events(
    pool: &SqlitePool,
    health_check_id: &str,
    limit: i64,
) -> Result<Vec<HealthCheckEvent>, DbError> {
    let events = sqlx::query_as::<_, HealthCheckEvent>(
        "SELECT * FROM health_check_events WHERE health_check_id = ? ORDER BY checked_at DESC LIMIT ?",
    )
    .bind(health_check_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(events)
}

pub(super) async fn get_health_events_for_checks(
    pool: &SqlitePool,
    health_check_ids: &[String],
    limit_per_check: i64,
) -> Result<Vec<HealthCheckEvent>, DbError> {
    if health_check_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: Vec<&str> = health_check_ids.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT * FROM health_check_events WHERE health_check_id IN ({}) \
         ORDER BY health_check_id, checked_at DESC",
        placeholders.join(", ")
    );

    let mut query = sqlx::query_as::<_, HealthCheckEvent>(&sql);
    for id in health_check_ids {
        query = query.bind(id);
    }

    let all_events = query.fetch_all(pool).await?;

    let mut result = Vec::with_capacity(all_events.len());
    let mut counts: std::collections::HashMap<&str, i64> = std::collections::HashMap::new();
    for event in &all_events {
        let count = counts.entry(&event.health_check_id).or_insert(0);
        if *count < limit_per_check {
            result.push(event.clone());
            *count += 1;
        }
    }

    Ok(result)
}
