use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::utils::{check_dns_points_to, detect_server_ip};
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings", get(get_settings))
        .route("/settings/runtime", get(get_runtime_info))
        .route("/settings/base-domain", post(setup_base_domain))
        .route("/settings/base-domain/verify", post(verify_base_domain))
        .route(
            "/settings/registration",
            get(get_registration_settings).put(update_registration_settings),
        )
        // Global reverse proxy settings (IF-149)
        .route(
            "/settings/proxy",
            get(get_proxy_settings).put(update_proxy_settings),
        )
        .route("/settings/proxy/config", get(get_full_proxy_config))
        .route("/settings/proxy/reload", post(reload_proxy))
}

async fn get_settings(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(serde_json::json!({
        "data": {
            "base_domain": state.config.base_domain,
            "version": env!("CARGO_PKG_VERSION"),
            "runtime": state.config.runtime.to_string(),
        }
    })))
}

async fn get_runtime_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers).await?;

    let runtime_info = state
        .docker
        .runtime_version()
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(serde_json::json!({
        "data": {
            "configured_runtime": state.config.runtime.to_string(),
            "detected_runtime": runtime_info.name,
            "version": runtime_info.version,
            "api_version": runtime_info.api_version,
            "os": runtime_info.os,
            "arch": runtime_info.arch,
            "socket": state.config.container_socket,
        }
    })))
}

#[derive(Deserialize)]
struct SetupBaseDomainRequest {
    base_domain: String,
}

async fn setup_base_domain(
    State(state): State<AppState>,
    Json(body): Json<SetupBaseDomainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let domain = body.base_domain.trim().to_lowercase();
    if domain.is_empty() {
        return Err(ApiError::BadRequest("base_domain is required".into()));
    }

    let server_ip = detect_server_ip().await;

    let wildcard = format!("*.{domain}");
    if let Err(e) = state.caddy.add_route(&wildcard, "localhost:0").await {
        tracing::warn!("Failed to configure Caddy wildcard for {wildcard}: {e}");
    }

    Ok(Json(serde_json::json!({
        "base_domain": domain,
        "dns_instructions": {
            "records": [
                {
                    "type": "A",
                    "name": &domain,
                    "value": server_ip.as_deref().unwrap_or("YOUR_SERVER_IP"),
                },
                {
                    "type": "A",
                    "name": format!("*.{domain}"),
                    "value": server_ip.as_deref().unwrap_or("YOUR_SERVER_IP"),
                }
            ],
            "note": "Create both A records pointing to your server. The wildcard record enables automatic subdomains for all apps."
        }
    })))
}

async fn verify_base_domain(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let base_domain = match &state.config.base_domain {
        Some(d) => d.clone(),
        None => {
            return Ok(Json(serde_json::json!({
                "configured": false,
                "error": "No base domain configured. Use POST /settings/base-domain first."
            })));
        }
    };

    let server_ip = detect_server_ip().await;
    let test_subdomain = format!("_icefall-verify.{base_domain}");

    let (base_ok, wildcard_ok) = tokio::join!(
        check_dns_points_to(&base_domain, server_ip.as_deref()),
        check_dns_points_to(&test_subdomain, server_ip.as_deref())
    );

    Ok(Json(serde_json::json!({
        "configured": true,
        "base_domain": base_domain,
        "server_ip": server_ip,
        "base_dns_ok": base_ok,
        "wildcard_dns_ok": wildcard_ok,
        "fully_verified": base_ok && wildcard_ok,
    })))
}

// --- Registration Settings ---

async fn get_registration_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let settings = state.db.get_registration_settings().await?;

    Ok(Json(serde_json::json!({
        "data": {
            "allow_registration": settings.allow_registration,
            "allowed_domains": settings.allowed_domains,
            "default_role": settings.default_role,
        }
    })))
}

#[derive(Deserialize)]
struct UpdateRegistrationSettingsRequest {
    allow_registration: Option<bool>,
    allowed_domains: Option<String>,
    default_role: Option<String>,
}

async fn update_registration_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateRegistrationSettingsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    // Get current settings as base
    let current = state.db.get_registration_settings().await?;

    let allow_registration = body
        .allow_registration
        .unwrap_or(current.allow_registration);
    let allowed_domains = body
        .allowed_domains
        .as_deref()
        .or(current.allowed_domains.as_deref());
    let default_role = body
        .default_role
        .as_deref()
        .unwrap_or(&current.default_role);

    if !["admin", "deployer", "viewer"].contains(&default_role) {
        return Err(ApiError::BadRequest(
            "default_role must be admin, deployer, or viewer".into(),
        ));
    }

    let updated = state
        .db
        .upsert_registration_settings(allow_registration, allowed_domains, default_role)
        .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "allow_registration": updated.allow_registration,
            "allowed_domains": updated.allowed_domains,
            "default_role": updated.default_role,
        },
        "message": "Registration settings updated"
    })))
}

// --- Global Reverse Proxy Settings (IF-149) ---

async fn get_proxy_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let settings = state.db.get_proxy_settings().await?;

    // Surface Caddy status alongside the stored defaults.
    let caddy_running = state.caddy.health_check().await.is_ok();

    Ok(Json(serde_json::json!({
        "data": {
            "default_headers": settings.default_headers,
            "default_rate_limit": settings.default_rate_limit,
            "force_https": settings.force_https,
            "public_port_range_start": settings.public_port_range_start,
            "public_port_range_end": settings.public_port_range_end,
            "updated_at": settings.updated_at,
            "caddy_running": caddy_running,
            "caddy_version": env!("CARGO_PKG_VERSION"),
        }
    })))
}

#[derive(Deserialize)]
struct UpdateProxySettingsRequest {
    /// JSON object of header name -> value. Omit to leave unchanged.
    default_headers: Option<serde_json::Value>,
    /// JSON: { enabled, requests, window, burst }. Omit to leave unchanged.
    default_rate_limit: Option<serde_json::Value>,
    force_https: Option<bool>,
    /// Public-port allocation range (IF-172). Both omitted = unchanged.
    public_port_range_start: Option<i32>,
    public_port_range_end: Option<i32>,
}

async fn update_proxy_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateProxySettingsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    // Validate the port range up front: a non-positive bound or an inverted range
    // would let the allocator hand out unusable ports. Missing sides use stored values.
    if body.public_port_range_start.is_some() || body.public_port_range_end.is_some() {
        let existing = state.db.get_proxy_settings().await?;
        let start = body
            .public_port_range_start
            .unwrap_or(existing.public_port_range_start);
        let end = body
            .public_port_range_end
            .unwrap_or(existing.public_port_range_end);
        // Ports above 65535 don't exist; below 1024 are privileged. Stay in the
        // unprivileged user range so Caddy can bind without elevated rights.
        if !(1024..=65535).contains(&start) || !(1024..=65535).contains(&end) {
            return Err(ApiError::BadRequest(
                "Public port range must be within 1024-65535".into(),
            ));
        }
        if start > end {
            return Err(ApiError::BadRequest(
                "Public port range start must not exceed end".into(),
            ));
        }
    }

    let update = crate::db::models::UpdateProxySettings {
        default_headers: body.default_headers.map(|v| Some(v.to_string())),
        default_rate_limit: body.default_rate_limit.map(|v| Some(v.to_string())),
        force_https: body.force_https,
        public_port_range_start: body.public_port_range_start,
        public_port_range_end: body.public_port_range_end,
    };

    let updated = state.db.update_proxy_settings(&update).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "default_headers": updated.default_headers,
            "default_rate_limit": updated.default_rate_limit,
            "force_https": updated.force_https,
            "public_port_range_start": updated.public_port_range_start,
            "public_port_range_end": updated.public_port_range_end,
            "updated_at": updated.updated_at,
        },
        "message": "Proxy settings updated"
    })))
}

async fn get_full_proxy_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let config = state.caddy.get_full_config().await?;
    Ok(Json(serde_json::json!({ "data": config })))
}

async fn reload_proxy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    // Re-apply the currently loaded config — a no-op reload that surfaces any
    // config errors and forces Caddy to pick up out-of-band changes.
    let config = state.caddy.get_full_config().await?;
    state
        .caddy
        .load_config(&config)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Caddy reload failed: {e}")))?;

    Ok(Json(serde_json::json!({
        "message": "Caddy config reloaded"
    })))
}
