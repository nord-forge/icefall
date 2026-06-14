use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewWebhookEndpoint;

#[derive(Deserialize)]
pub struct CreateWebhookRequest {
    name: String,
    url: String,
    method: Option<String>,
    secret: Option<String>,
    headers: Option<String>,
}

pub async fn list_endpoints(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let endpoints = state.db.list_webhook_endpoints().await?;
    Ok(Json(serde_json::json!({ "data": endpoints })))
}

pub async fn create_endpoint(
    State(state): State<AppState>,
    Json(body): Json<CreateWebhookRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let endpoint = state
        .db
        .create_webhook_endpoint(&NewWebhookEndpoint {
            name: body.name,
            url: body.url,
            method: body.method,
            secret: body.secret,
            headers: body.headers,
        })
        .await?;
    Ok(Json(serde_json::json!({ "data": endpoint })))
}

pub async fn delete_endpoint(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.delete_webhook_endpoint(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

pub async fn list_deliveries(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deliveries = state.db.list_webhook_deliveries(&id, 50).await?;
    Ok(Json(serde_json::json!({ "data": deliveries })))
}

pub async fn test_endpoint(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let endpoints = state.db.list_webhook_endpoints().await?;
    let endpoint = endpoints
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("webhook endpoint {id}")))?;

    // Block SSRF — the endpoint URL is user-supplied. The guarded client is
    // pinned to the validated IP and refuses redirects.
    let target = crate::api::utils::url_guard::validate_outbound_url(
        &endpoint.url,
        &state.config.caddy_admin_url,
    )
    .await?;

    let payload = serde_json::json!({
        "event": "test",
        "summary": "Test webhook delivery",
        "details": {},
        "timestamp": crate::db::models::now_iso8601(),
    });

    let client = crate::api::utils::url_guard::guarded_client(&target)?;
    let mut request = client
        .request(reqwest::Method::POST, target.url.clone())
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10));

    if let Some(ref secret) = endpoint.secret {
        let body_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let signature = compute_hmac_signature(secret, &body_bytes);
        request = request.header("X-Icefall-Signature", format!("sha256={signature}"));
    }

    let start = std::time::Instant::now();
    let result = request.send().await;
    let response_time_ms = start.elapsed().as_millis() as i32;

    match result {
        Ok(resp) => {
            let status_code = resp.status().as_u16() as i32;
            let _ = state
                .db
                .create_webhook_delivery(
                    &id,
                    "test",
                    Some(status_code),
                    Some(response_time_ms),
                    1,
                    None,
                )
                .await;
            Ok(Json(serde_json::json!({
                "data": { "status_code": status_code, "response_time_ms": response_time_ms }
            })))
        }
        Err(e) => {
            let _ = state
                .db
                .create_webhook_delivery(
                    &id,
                    "test",
                    None,
                    Some(response_time_ms),
                    1,
                    Some(&e.to_string()),
                )
                .await;
            Ok(Json(serde_json::json!({
                "data": { "error": e.to_string(), "response_time_ms": response_time_ms }
            })))
        }
    }
}

fn compute_hmac_signature(secret: &str, body: &[u8]) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let mut mac =
        <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}
