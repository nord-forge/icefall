use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn record_drift_event(
    pool: &SqlitePool,
    app_id: &str,
    drifted_fields: &str,
    declared: Option<&str>,
    actual: Option<&str>,
) -> Result<DriftEvent, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO drift_events (id, app_id, drifted_fields, declared_state, actual_state, resolved, detected_at)
         VALUES (?, ?, ?, ?, ?, FALSE, ?)",
    )
    .bind(&id)
    .bind(app_id)
    .bind(drifted_fields)
    .bind(declared)
    .bind(actual)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(DriftEvent {
        id,
        app_id: app_id.to_string(),
        drifted_fields: drifted_fields.to_string(),
        declared_state: declared.map(String::from),
        actual_state: actual.map(String::from),
        resolved: false,
        detected_at: now,
    })
}

pub(super) async fn list_drift_events(
    pool: &SqlitePool,
    app_id: &str,
    limit: i64,
) -> Result<Vec<DriftEvent>, DbError> {
    let events = sqlx::query_as::<_, DriftEvent>(
        "SELECT * FROM drift_events WHERE app_id = ? ORDER BY detected_at DESC LIMIT ?",
    )
    .bind(app_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(events)
}

pub(super) async fn resolve_drift_event(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE drift_events SET resolved = TRUE WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
