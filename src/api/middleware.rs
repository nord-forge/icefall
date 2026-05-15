use axum::body::Body;
use axum::http::{HeaderName, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::api::AppState;
use crate::config::IcefallConfig;

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

const PUBLIC_PATHS: &[&str] = &[
    "/api/v1/auth/login",
    "/api/v1/auth/register",
    "/api/v1/auth/setup",
    "/api/v1/auth/forgot-password",
    "/api/v1/auth/reset-password",
    "/api/v1/health",
    "/api/v1/servers/setup",
    "/api/v1/settings/oauth/providers",
    "/api/v1/github/events",
    "/api/v1/github/callback",
];

const PUBLIC_PREFIXES: &[&str] = &[
    "/api/v1/webhooks/source/",
    "/api/v1/agent/",
    "/api/v1/oauth/",
    "/api/v1/onboarding",
    "/api/v1/status/",
    "/api/v1/invitations/",
];

fn is_public_path(path: &str) -> bool {
    if PUBLIC_PATHS.contains(&path) {
        return true;
    }
    if PUBLIC_PREFIXES.iter().any(|p| path.starts_with(p)) {
        return true;
    }
    // Terminal and SSE endpoints handle their own auth
    if path.contains("/terminal") || path.contains("/events") {
        return true;
    }
    // Non-API paths (dashboard static files)
    if !path.starts_with("/api/") {
        return true;
    }
    false
}

pub async fn require_auth(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    if is_public_path(&path) {
        return next.run(req).await;
    }

    let headers = req.headers();

    // Check Bearer token (API token or session ID)
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Bearer icefall_") {
                let token = auth_str.trim_start_matches("Bearer ");
                let hash = sha256_hex(token);
                if let Ok(Some(api_token)) = state.db.get_api_token_by_hash(&hash).await {
                    let expired = api_token
                        .expires_at
                        .as_ref()
                        .is_some_and(|exp| exp < &crate::db::models::now_iso8601());
                    if !expired {
                        let _ = state.db.update_token_last_used(&api_token.id).await;
                        return next.run(req).await;
                    }
                }
            } else if let Some(session_id) = auth_str.strip_prefix("Bearer ") {
                if let Ok(Some(session)) = state.db.get_session(session_id).await {
                    if session.expires_at >= crate::db::models::now_iso8601() {
                        return next.run(req).await;
                    }
                }
            }
        }
    }

    // Check session cookie
    if let Some(cookie) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some(session_id) = part.strip_prefix("icefall_session=") {
                    if let Ok(Some(session)) = state.db.get_session(session_id).await {
                        if session.expires_at >= crate::db::models::now_iso8601() {
                            return next.run(req).await;
                        }
                    }
                }
            }
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({
            "error": { "code": "unauthorized", "message": "Authentication required" }
        })),
    )
        .into_response()
}

fn sha256_hex(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn apply_middleware(router: Router<AppState>, _config: &IcefallConfig) -> Router<AppState> {
    router
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID.clone()))
        .layer(SetRequestIdLayer::new(
            X_REQUEST_ID.clone(),
            MakeRequestUuid,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
