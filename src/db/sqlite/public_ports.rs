use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn allocate_public_port(
    pool: &SqlitePool,
    resource_type: &str,
    resource_id: &str,
    port: i32,
    ip_whitelist: Option<&str>,
) -> Result<PublicPort, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO public_ports (id, resource_type, resource_id, port, ip_whitelist, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(port)
    .bind(ip_whitelist)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            DbError::Duplicate(format!("Port {port} is already allocated"))
        }
        other => DbError::Sqlx(other),
    })?;

    Ok(PublicPort {
        id,
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
        port,
        ip_whitelist: ip_whitelist.map(String::from),
        created_at: now,
    })
}

pub(super) async fn release_public_port(
    pool: &SqlitePool,
    resource_id: &str,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM public_ports WHERE resource_id = ?")
        .bind(resource_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn get_public_port(
    pool: &SqlitePool,
    resource_id: &str,
) -> Result<Option<PublicPort>, DbError> {
    let port = sqlx::query_as::<_, PublicPort>("SELECT * FROM public_ports WHERE resource_id = ?")
        .bind(resource_id)
        .fetch_optional(pool)
        .await?;
    Ok(port)
}
