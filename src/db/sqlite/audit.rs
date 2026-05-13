use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_audit_log(
    pool: &SqlitePool,
    entry: &NewAuditLogEntry,
) -> Result<(), DbError> {
    let id = new_id();
    let details = serde_json::to_string(&entry.details).unwrap_or_else(|_| "{}".into());
    sqlx::query(
        "INSERT INTO audit_log (id, server_id, user_id, action, details, ip_address) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&entry.server_id)
    .bind(&entry.user_id)
    .bind(&entry.action)
    .bind(&details)
    .bind(&entry.ip_address)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn list_audit_logs(
    pool: &SqlitePool,
    server_id: Option<&str>,
    action: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<Vec<AuditLogEntry>, DbError> {
    let mut sql = "SELECT * FROM audit_log WHERE 1=1".to_string();
    if server_id.is_some() {
        sql.push_str(" AND server_id = ?");
    }
    if action.is_some() {
        sql.push_str(" AND action = ?");
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

    let mut query = sqlx::query_as::<_, AuditLogEntry>(&sql);
    if let Some(sid) = server_id {
        query = query.bind(sid);
    }
    if let Some(act) = action {
        query = query.bind(act);
    }
    query = query.bind(limit).bind(offset);

    let entries = query.fetch_all(pool).await?;
    Ok(entries)
}

pub(super) async fn prune_audit_logs(pool: &SqlitePool, older_than: &str) -> Result<u64, DbError> {
    let result = sqlx::query("DELETE FROM audit_log WHERE created_at < ?")
        .bind(older_than)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
