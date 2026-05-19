use std::sync::LazyLock;

use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::api::rate_limit;
use crate::api::AppState;
use crate::config::IcefallConfig;

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Custom header the dashboard sends on every request — a lightweight CSRF
/// defence: cross-site forms/images can't set custom headers, only same-origin fetch/XHR.
static X_ICEFALL_REQUEST: HeaderName = HeaderName::from_static("x-icefall-request");

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
    // Onboarding: only status + first-admin creation are public; every other
    // onboarding endpoint is gated behind auth. create_admin has its own empty-users guard.
    "/api/v1/onboarding/status",
    "/api/v1/onboarding/admin",
];

const PUBLIC_PREFIXES: &[&str] = &[
    "/api/v1/webhooks/source/",
    "/api/v1/agent/",
    "/api/v1/status/",
    "/api/v1/invitations/",
];

/// OAuth browser-redirect endpoints reachable without a session (the callback is
/// what creates it). `link`/`unlink`/`identities` are NOT here — they require auth.
fn is_public_oauth_path(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("/api/v1/auth/oauth/") else {
        return false;
    };
    // rest is "{provider}/authorize" or "{provider}/callback"
    rest.split_once('/')
        .is_some_and(|(_, action)| action == "authorize" || action == "callback")
}

fn is_public_path(path: &str) -> bool {
    if PUBLIC_PATHS.contains(&path) {
        return true;
    }
    if PUBLIC_PREFIXES.iter().any(|p| path.starts_with(p)) {
        return true;
    }
    if is_public_oauth_path(path) {
        return true;
    }
    // Terminal and SSE endpoints handle their own auth. Suffix matching (not
    // `contains`) so an arbitrary path can't smuggle past auth by including the substring.
    if path.ends_with("/terminal") || path.ends_with("/events") {
        return true;
    }
    // Non-API paths (dashboard static files).
    if !path.starts_with("/api/") {
        return true;
    }
    false
}

/// Whether an HTTP method changes server state (and therefore needs the CSRF
/// header check and per-user rate limiting on the mutating path).
fn is_mutating(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

pub async fn require_auth(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let headers = req.headers();

    // CSRF defence: mutating requests must carry the X-Icefall-Request header.
    // Webhook/agent callbacks are exempt — machine-to-machine, authenticated via signatures/tokens.
    if is_mutating(&method)
        && path.starts_with("/api/")
        && !is_public_path(&path)
        && headers.get(&X_ICEFALL_REQUEST).is_none()
    {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "forbidden",
                "message": "Missing X-Icefall-Request header"
            })),
        )
            .into_response();
    }

    if is_public_path(&path) {
        return next.run(req).await;
    }

    // Authenticated path: identify the caller, then apply a per-user rate
    // limit before running the handler (audit H1).
    let user_id = resolve_user_id(&state, headers).await;

    let Some(user_id) = user_id else {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "unauthorized",
                "message": "Authentication required"
            })),
        )
            .into_response();
    };

    if is_mutating(&method) && !rate_limit::API_PER_USER.check(&user_id).await {
        return too_many_requests();
    }

    next.run(req).await
}

/// Resolve the authenticated user's ID from a Bearer token (API token or
/// session) or the session cookie. Returns `None` if not authenticated.
async fn resolve_user_id(state: &AppState, headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            if token.starts_with("icefall_") {
                let hash = sha256_hex(token);
                if let Ok(Some(api_token)) = state.db.get_api_token_by_hash(&hash).await {
                    let expired = api_token
                        .expires_at
                        .as_ref()
                        .is_some_and(|exp| exp < &crate::db::models::now_iso8601());
                    if !expired {
                        let _ = state.db.update_token_last_used(&api_token.id).await;
                        return Some(api_token.user_id);
                    }
                }
            } else if let Ok(Some(session)) = state.db.get_session(token).await {
                if session.expires_at >= crate::db::models::now_iso8601() {
                    return Some(session.user_id);
                }
            }
        }
    }

    if let Some(cookie_str) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        for part in cookie_str.split(';') {
            if let Some(session_id) = part.trim().strip_prefix("icefall_session=") {
                if let Ok(Some(session)) = state.db.get_session(session_id).await {
                    if session.expires_at >= crate::db::models::now_iso8601() {
                        return Some(session.user_id);
                    }
                }
            }
        }
    }

    None
}

fn too_many_requests() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        axum::Json(serde_json::json!({
            "error": "too_many_requests",
            "message": "Rate limit exceeded. Please slow down."
        })),
    )
        .into_response()
}

/// Global per-IP rate-limit layer — a safety net in front of every route
/// (audit H1). Tighter per-endpoint limits live on the auth handlers.
pub async fn global_rate_limit(req: Request<Body>, next: Next) -> Response {
    let ip = rate_limit::client_ip(req.headers());
    if !rate_limit::GLOBAL.check(&ip).await {
        return too_many_requests();
    }
    next.run(req).await
}

fn sha256_hex(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Content-Security-Policy: hash-pinned inline `script-src`, `unsafe-inline` `style-src`
/// for Astro. Hashes come from `csp-hashes.json`; if absent, inline scripts are omitted.
static CSP_VALUE: LazyLock<String> = LazyLock::new(|| {
    let hashes = load_csp_script_hashes();
    let script_src = if hashes.is_empty() {
        "script-src 'self'".to_string()
    } else {
        format!("script-src 'self' {}", hashes.join(" "))
    };
    [
        "default-src 'self'",
        script_src.as_str(),
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data:",
        "font-src 'self'",
        "connect-src 'self'",
        "frame-ancestors 'none'",
        "base-uri 'self'",
        "object-src 'none'",
        "form-action 'self'",
    ]
    .join("; ")
});

fn load_csp_script_hashes() -> Vec<String> {
    // The dashboard build (`dashboard/scripts/csp-hashes.mjs`) writes this
    // file beside the served HTML in `dashboard/dist`.
    let path = "dashboard/dist/csp-hashes.json";
    if let Ok(contents) = std::fs::read_to_string(path) {
        if let Ok(hashes) = serde_json::from_str::<Vec<String>>(&contents) {
            return hashes.into_iter().map(|h| format!("'{h}'")).collect();
        }
    }
    tracing::warn!(
        "csp-hashes.json not found — CSP will block inline scripts; run the dashboard build"
    );
    Vec::new()
}

/// Static security headers applied to every response (audit H3).
fn security_header_layers(config: &IcefallConfig) -> Vec<SetResponseHeaderLayer<HeaderValue>> {
    let mut headers: Vec<(HeaderName, &str)> = vec![
        (HeaderName::from_static("x-content-type-options"), "nosniff"),
        (HeaderName::from_static("x-frame-options"), "DENY"),
        (
            HeaderName::from_static("referrer-policy"),
            "strict-origin-when-cross-origin",
        ),
        (
            HeaderName::from_static("permissions-policy"),
            "camera=(), microphone=(), geolocation=()",
        ),
        // Modern browsers ignore X-XSS-Protection; 0 explicitly disables the
        // legacy auditor, which could itself introduce vulnerabilities.
        (HeaderName::from_static("x-xss-protection"), "0"),
    ];

    let mut layers: Vec<SetResponseHeaderLayer<HeaderValue>> = headers
        .drain(..)
        .filter_map(|(name, value)| {
            HeaderValue::from_str(value)
                .ok()
                .map(|v| SetResponseHeaderLayer::overriding(name, v))
        })
        .collect();

    // CSP — built once from the hash file.
    if let Ok(csp) = HeaderValue::from_str(&CSP_VALUE) {
        layers.push(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("content-security-policy"),
            csp,
        ));
    }

    // HSTS only when a base_domain is configured — that's when behind Caddy with TLS.
    // Sending it over plain HTTP (local dev) would wrongly pin the browser to HTTPS.
    if config.base_domain.is_some() {
        if let Ok(hsts) = HeaderValue::from_str("max-age=31536000; includeSubDomains") {
            layers.push(SetResponseHeaderLayer::overriding(
                HeaderName::from_static("strict-transport-security"),
                hsts,
            ));
        }
    }

    layers
}

/// Build the CORS layer, restricted to an explicit allowlist (prod base_domain, or
/// localhost in dev) — `allow_credentials` for cookie auth is incompatible with a wildcard.
fn cors_layer(config: &IcefallConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = match config.base_domain.as_deref() {
        Some(domain) => vec![format!("https://{domain}")],
        None => {
            let port = config.listen_port;
            vec![
                format!("http://localhost:{port}"),
                format!("http://127.0.0.1:{port}"),
            ]
        }
    }
    .into_iter()
    .filter_map(|o| HeaderValue::from_str(&o).ok())
    .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            X_ICEFALL_REQUEST.clone(),
        ])
        .allow_credentials(true)
}

pub fn apply_middleware(router: Router<AppState>, config: &IcefallConfig) -> Router<AppState> {
    let mut router = router
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID.clone()))
        .layer(SetRequestIdLayer::new(
            X_REQUEST_ID.clone(),
            MakeRequestUuid,
        ))
        .layer(axum::middleware::from_fn(global_rate_limit))
        .layer(cors_layer(config));

    for layer in security_header_layers(config) {
        router = router.layer(layer);
    }
    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_paths_are_public() {
        assert!(is_public_path("/api/v1/auth/login"));
        assert!(is_public_path("/api/v1/health"));
        assert!(is_public_path("/api/v1/onboarding/status"));
        assert!(is_public_path("/api/v1/onboarding/admin"));
    }

    #[test]
    fn onboarding_endpoints_other_than_status_and_admin_are_gated() {
        // audit H4: post-setup onboarding endpoints must require auth.
        assert!(!is_public_path("/api/v1/onboarding/domain"));
        assert!(!is_public_path("/api/v1/onboarding/app"));
        assert!(!is_public_path("/api/v1/onboarding/complete"));
    }

    #[test]
    fn substring_terminal_does_not_bypass_auth() {
        // audit M3: only an exact suffix, not a substring, is public.
        assert!(is_public_path("/api/v1/apps/abc/terminal"));
        assert!(!is_public_path("/api/v1/terminal/secret/apps"));
        assert!(!is_public_path("/api/v1/apps/terminal-logs"));
    }

    #[test]
    fn oauth_authorize_and_callback_are_public_but_link_is_not() {
        assert!(is_public_oauth_path("/api/v1/auth/oauth/github/authorize"));
        assert!(is_public_oauth_path("/api/v1/auth/oauth/google/callback"));
        assert!(!is_public_oauth_path("/api/v1/auth/oauth/github/link"));
        assert!(!is_public_oauth_path("/api/v1/auth/oauth/identities"));
        assert!(!is_public_oauth_path("/api/v1/auth/oauth/github/unlink"));
    }

    #[test]
    fn mutating_methods_detected() {
        assert!(is_mutating(&Method::POST));
        assert!(is_mutating(&Method::PUT));
        assert!(is_mutating(&Method::PATCH));
        assert!(is_mutating(&Method::DELETE));
        assert!(!is_mutating(&Method::GET));
        assert!(!is_mutating(&Method::HEAD));
        assert!(!is_mutating(&Method::OPTIONS));
    }
}
