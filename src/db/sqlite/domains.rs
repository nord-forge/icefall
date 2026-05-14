use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn add_domain(pool: &SqlitePool, domain: &NewDomain) -> Result<Domain, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO domains (id, app_id, domain, path, verified, ssl_status, created_at)
         VALUES (?, ?, ?, ?, FALSE, 'pending', ?)",
    )
    .bind(&id)
    .bind(&domain.app_id)
    .bind(&domain.domain)
    .bind(&domain.path)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(Domain {
        id,
        app_id: domain.app_id.clone(),
        domain: domain.domain.clone(),
        path: domain.path.clone(),
        verified: false,
        ssl_status: "pending".to_string(),
        ssl_issuer: None,
        ssl_expires_at: None,
        ssl_last_checked_at: None,
        created_at: now,
    })
}

pub(super) async fn list_domains(pool: &SqlitePool, app_id: &str) -> Result<Vec<Domain>, DbError> {
    let domains =
        sqlx::query_as::<_, Domain>("SELECT * FROM domains WHERE app_id = ? ORDER BY created_at")
            .bind(app_id)
            .fetch_all(pool)
            .await?;
    Ok(domains)
}

pub(super) async fn update_domain_status(
    pool: &SqlitePool,
    id: &str,
    verified: bool,
    ssl_status: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE domains SET verified = ?, ssl_status = ? WHERE id = ?")
        .bind(verified)
        .bind(ssl_status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn delete_domain(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM domains WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn list_all_domains(pool: &SqlitePool) -> Result<Vec<Domain>, DbError> {
    let domains = sqlx::query_as::<_, Domain>("SELECT * FROM domains WHERE verified = TRUE")
        .fetch_all(pool)
        .await?;
    Ok(domains)
}

pub(super) async fn update_domain_ssl_info(
    pool: &SqlitePool,
    id: &str,
    issuer: Option<&str>,
    expires_at: Option<&str>,
) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query(
        "UPDATE domains SET ssl_issuer = ?, ssl_expires_at = ?, ssl_last_checked_at = ? WHERE id = ?",
    )
    .bind(issuer)
    .bind(expires_at)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
