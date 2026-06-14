use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_incident(
    pool: &SqlitePool,
    incident: &NewIncident,
) -> Result<Incident, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO incidents (id, title, status, severity, affected_apps, affected_servers, started_at, created_at, updated_at)
         VALUES (?, ?, 'investigating', ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&incident.title)
    .bind(&incident.severity)
    .bind(&incident.affected_apps)
    .bind(&incident.affected_servers)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(DbError::from)
}

pub(super) async fn list_incidents(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<Incident>, DbError> {
    let incidents =
        sqlx::query_as::<_, Incident>("SELECT * FROM incidents ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await?;
    Ok(incidents)
}

pub(super) async fn update_incident_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
) -> Result<(), DbError> {
    let now = now_iso8601();
    let resolved_at = if status == "resolved" {
        Some(now.clone())
    } else {
        None
    };

    sqlx::query(
        "UPDATE incidents SET status = ?, resolved_at = COALESCE(?, resolved_at), updated_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(&resolved_at)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn add_incident_note(
    pool: &SqlitePool,
    incident_id: &str,
    content: &str,
    author_id: Option<&str>,
) -> Result<IncidentNote, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO incident_notes (id, incident_id, content, author_id, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(incident_id)
    .bind(content)
    .bind(author_id)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(IncidentNote {
        id,
        incident_id: incident_id.to_string(),
        content: content.to_string(),
        author_id: author_id.map(String::from),
        created_at: now,
    })
}
