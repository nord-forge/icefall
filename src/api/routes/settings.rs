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
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
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
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
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
