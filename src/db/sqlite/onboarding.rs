use sqlx::SqlitePool;

use crate::db::DbError;

pub(super) async fn get_onboarding(
    pool: &SqlitePool,
) -> Result<Option<(String, String, String, Option<String>)>, DbError> {
    let row = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT current_step, completed_steps, started_at, completed_at FROM onboarding WHERE id = 'singleton'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub(super) async fn create_onboarding(pool: &SqlitePool, started_at: &str) -> Result<(), DbError> {
    sqlx::query("INSERT OR IGNORE INTO onboarding (id, current_step, completed_steps, started_at) VALUES ('singleton', 'admin_account', '[]', ?)")
        .bind(started_at)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_onboarding_state(
    pool: &SqlitePool,
    current_step: &str,
    completed_steps: &str,
    completed_at: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE onboarding SET current_step = ?, completed_steps = ?, completed_at = ? WHERE id = 'singleton'")
        .bind(current_step)
        .bind(completed_steps)
        .bind(completed_at)
        .execute(pool)
        .await?;
    Ok(())
}
