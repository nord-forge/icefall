use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn get_cleanup_schedule(
    pool: &SqlitePool,
) -> Result<Option<CleanupSchedule>, DbError> {
    let schedule = sqlx::query_as::<_, CleanupSchedule>(
        "SELECT * FROM cleanup_schedule WHERE id = 'singleton'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(schedule)
}

pub(super) async fn upsert_cleanup_schedule(
    pool: &SqlitePool,
    schedule: &CleanupSchedule,
) -> Result<CleanupSchedule, DbError> {
    let now = now_iso8601();
    sqlx::query(
        "INSERT INTO cleanup_schedule (id, enabled, cron_expression, disk_threshold_percent, cleanup_dangling_images, cleanup_unused_images, cleanup_stopped_containers, cleanup_unused_volumes, cleanup_unused_networks, stopped_container_age_hours, updated_at)
         VALUES ('singleton', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            enabled = excluded.enabled,
            cron_expression = excluded.cron_expression,
            disk_threshold_percent = excluded.disk_threshold_percent,
            cleanup_dangling_images = excluded.cleanup_dangling_images,
            cleanup_unused_images = excluded.cleanup_unused_images,
            cleanup_stopped_containers = excluded.cleanup_stopped_containers,
            cleanup_unused_volumes = excluded.cleanup_unused_volumes,
            cleanup_unused_networks = excluded.cleanup_unused_networks,
            stopped_container_age_hours = excluded.stopped_container_age_hours,
            updated_at = excluded.updated_at",
    )
    .bind(schedule.enabled)
    .bind(&schedule.cron_expression)
    .bind(schedule.disk_threshold_percent)
    .bind(schedule.cleanup_dangling_images)
    .bind(schedule.cleanup_unused_images)
    .bind(schedule.cleanup_stopped_containers)
    .bind(schedule.cleanup_unused_volumes)
    .bind(schedule.cleanup_unused_networks)
    .bind(schedule.stopped_container_age_hours)
    .bind(&now)
    .execute(pool)
    .await?;

    get_cleanup_schedule(pool)
        .await?
        .ok_or_else(|| DbError::NotFound("cleanup_schedule".to_string()))
}
