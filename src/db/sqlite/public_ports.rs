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

/// Allocate the lowest free port in `[range_start, range_end]` for a resource.
/// `UNIQUE(port)` is the source of truth; racers retry the next free port.
pub(super) async fn allocate_free_public_port(
    pool: &SqlitePool,
    resource_type: &str,
    resource_id: &str,
    range_start: i32,
    range_end: i32,
    ip_whitelist: Option<&str>,
) -> Result<PublicPort, DbError> {
    if range_start > range_end {
        return Err(DbError::InvalidInput(format!(
            "invalid public port range {range_start}-{range_end}"
        )));
    }

    // Bound the retry loop by the range size: each iteration claims a distinct
    // port, so after that many attempts the range is genuinely full.
    let max_attempts = (range_end - range_start + 1) as usize;
    for _ in 0..max_attempts {
        // Lowest port in range not already present in public_ports.
        let taken: Vec<i32> = sqlx::query_scalar(
            "SELECT port FROM public_ports WHERE port BETWEEN ? AND ? ORDER BY port",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_all(pool)
        .await?;

        let candidate = (range_start..=range_end).find(|p| !taken.contains(p));
        let Some(port) = candidate else {
            break;
        };

        match allocate_public_port(pool, resource_type, resource_id, port, ip_whitelist).await {
            Ok(allocated) => return Ok(allocated),
            // Lost a race for this port — recompute and try the next free one.
            Err(DbError::Duplicate(_)) => continue,
            Err(other) => return Err(other),
        }
    }

    Err(DbError::InvalidInput(format!(
        "no free public ports in range {range_start}-{range_end}"
    )))
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
