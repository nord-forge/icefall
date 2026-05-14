use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn list_webhook_endpoints(
    pool: &SqlitePool,
) -> Result<Vec<WebhookEndpoint>, DbError> {
    let endpoints = sqlx::query_as::<_, WebhookEndpoint>(
        "SELECT * FROM webhook_endpoints ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(endpoints)
}

pub(super) async fn create_webhook_endpoint(
    pool: &SqlitePool,
    endpoint: &NewWebhookEndpoint,
) -> Result<WebhookEndpoint, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let method = endpoint.method.as_deref().unwrap_or("POST");

    sqlx::query(
        "INSERT INTO webhook_endpoints (id, name, url, method, secret, headers, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, TRUE, ?, ?)",
    )
    .bind(&id)
    .bind(&endpoint.name)
    .bind(&endpoint.url)
    .bind(method)
    .bind(&endpoint.secret)
    .bind(&endpoint.headers)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, WebhookEndpoint>("SELECT * FROM webhook_endpoints WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(DbError::from)
}

pub(super) async fn delete_webhook_endpoint(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM webhook_endpoints WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn create_webhook_delivery(
    pool: &SqlitePool,
    endpoint_id: &str,
    event: &str,
    status_code: Option<i32>,
    response_time_ms: Option<i32>,
    attempt: i32,
    error: Option<&str>,
) -> Result<(), DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO webhook_deliveries (id, endpoint_id, event, status_code, response_time_ms, attempt, error, delivered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(endpoint_id)
    .bind(event)
    .bind(status_code)
    .bind(response_time_ms)
    .bind(attempt)
    .bind(error)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn list_webhook_deliveries(
    pool: &SqlitePool,
    endpoint_id: &str,
    limit: i64,
) -> Result<Vec<WebhookDelivery>, DbError> {
    let deliveries = sqlx::query_as::<_, WebhookDelivery>(
        "SELECT * FROM webhook_deliveries WHERE endpoint_id = ? ORDER BY delivered_at DESC LIMIT ?",
    )
    .bind(endpoint_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(deliveries)
}
