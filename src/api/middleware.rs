use std::sync::LazyLock;

use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Router;
use tower_http::compression::CompressionLayer;
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
    let Some(caller) = resolve_caller(&state, headers).await else {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "unauthorized",
                "message": "Authentication required"
            })),
        )
            .into_response();
    };

    // IF-168: enforce API token ability scoping. A token with non-null
    // abilities may only reach routes whose required ability it was granted.
    if let Some(abilities) = &caller.token_abilities {
        if let Some(required) = crate::api::abilities::required_ability(&method, &path) {
            if !crate::api::abilities::granted(abilities, &required) {
                return (
                    StatusCode::FORBIDDEN,
                    axum::Json(serde_json::json!({
                        "error": {
                            "code": "insufficient_scope",
                            "message": format!("Token lacks '{required}' ability")
                        }
                    })),
                )
                    .into_response();
            }
        }
    }

    if is_mutating(&method) && !rate_limit::API_PER_USER.check(&caller.user_id).await {
        return too_many_requests();
    }

    next.run(req).await
}

/// An authenticated caller plus, for ability-scoped API tokens, the granted
/// scopes. `token_abilities` is `None` for session/cookie auth or a token with
/// null abilities (both = full access).
struct Caller {
    user_id: String,
    token_abilities: Option<Vec<String>>,
}

/// Resolve the authenticated caller from a Bearer token (API token or session)
/// or the session cookie. Returns `None` if not authenticated. For ability-
/// scoped API tokens the granted scopes are returned for enforcement.
async fn resolve_caller(state: &AppState, headers: &axum::http::HeaderMap) -> Option<Caller> {
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
                        // null abilities = full access; otherwise parse the
                        // granted scopes (a malformed value denies everything).
                        let token_abilities = api_token.abilities.as_deref().map(|json| {
                            serde_json::from_str::<Vec<String>>(json).unwrap_or_default()
                        });
                        return Some(Caller {
                            user_id: api_token.user_id,
                            token_abilities,
                        });
                    }
                }
            } else if let Ok(Some(session)) = state.db.get_session(token).await {
                if session.expires_at >= crate::db::models::now_iso8601() {
                    return Some(Caller {
                        user_id: session.user_id,
                        token_abilities: None,
                    });
                }
            }
        }
    }

    if let Some(cookie_str) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        for part in cookie_str.split(';') {
            if let Some(session_id) = part.trim().strip_prefix("icefall_session=") {
                if let Ok(Some(session)) = state.db.get_session(session_id).await {
                    if session.expires_at >= crate::db::models::now_iso8601() {
                        return Some(Caller {
                            user_id: session.user_id,
                            token_abilities: None,
                        });
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
    // The dashboard build (`dashboard/scripts/csp-hashes.mjs`) writes
    // `csp-hashes.json` into dist, which is embedded in the binary (IF-255).
    if let Some(contents) = crate::api::assets::csp_hashes_json() {
        if let Ok(hashes) = serde_json::from_str::<Vec<String>>(&contents) {
            return hashes.into_iter().map(|h| format!("'{h}'")).collect();
        }
    }
    tracing::warn!(
        "csp-hashes.json not found in embedded dashboard — CSP will block inline scripts"
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

/// `Cache-Control` value for a request path (IF-253).
///
/// Astro content-hashes asset filenames (`Alert.CXSNITkF.js`), so anything under
/// `/_astro/` is immutable and can be cached for a year. HTML shells must NOT be
/// cached long — they reference the hashed assets, and a deploy changes those
/// hashes, so the shell has to be re-fetched to discover them. API responses are
/// dynamic and left uncached. Returns `None` when no header should be set.
fn cache_control_for_path(path: &str) -> Option<&'static str> {
    if path.starts_with("/api/") {
        return None;
    }
    if path.starts_with("/_astro/") {
        // Immutable, content-hashed assets.
        Some("public, max-age=31536000, immutable")
    } else {
        // HTML shells and other dashboard files — always revalidate so updates
        // (new asset hashes) are picked up immediately.
        Some("no-cache")
    }
}

/// Set `Cache-Control` on dashboard responses based on the request path. Only
/// adds the header when the response doesn't already carry one, so a handler
/// that sets its own caching wins.
async fn dashboard_cache_control(req: Request<Body>, next: Next) -> Response {
    let cache_value = cache_control_for_path(req.uri().path());
    let mut response = next.run(req).await;
    if let Some(value) = cache_value {
        let headers = response.headers_mut();
        if !headers.contains_key(axum::http::header::CACHE_CONTROL) {
            if let Ok(hv) = HeaderValue::from_str(value) {
                headers.insert(axum::http::header::CACHE_CONTROL, hv);
            }
        }
    }
    response
}

pub fn apply_middleware(router: Router<AppState>, config: &IcefallConfig) -> Router<AppState> {
    let mut router = router
        .layer(TraceLayer::new_for_http())
        // Compress responses (gzip/br) — the dashboard ships ~900 KB of JS that
        // was previously sent uncompressed (IF-252). Negotiated per
        // Accept-Encoding; clients that don't ask get identity.
        .layer(CompressionLayer::new())
        // Path-based Cache-Control for dashboard assets (IF-253).
        .layer(axum::middleware::from_fn(dashboard_cache_control))
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
    fn cache_control_policy() {
        // Content-hashed assets are immutable.
        assert_eq!(
            cache_control_for_path("/_astro/Alert.CXSNITkF.js"),
            Some("public, max-age=31536000, immutable")
        );
        // HTML shells must revalidate so new asset hashes are discovered.
        assert_eq!(cache_control_for_path("/"), Some("no-cache"));
        assert_eq!(cache_control_for_path("/apps/abc"), Some("no-cache"));
        assert_eq!(cache_control_for_path("/index.html"), Some("no-cache"));
        // API responses are dynamic — never cached by this layer.
        assert_eq!(cache_control_for_path("/api/v1/apps"), None);
    }

    // Drive the cache-control middleware through a real router to confirm it
    // actually sets the response header for the matched path.
    async fn cache_header_for(path: &'static str) -> Option<String> {
        use axum::routing::get;
        use tower::ServiceExt; // for `oneshot`

        let app = Router::new()
            .route("/{*rest}", get(|| async { "body" }))
            .route("/", get(|| async { "body" }))
            .layer(axum::middleware::from_fn(dashboard_cache_control));

        let req = Request::builder().uri(path).body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        resp.headers()
            .get(axum::http::header::CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[tokio::test]
    async fn cache_control_header_is_set_on_responses() {
        assert_eq!(
            cache_header_for("/_astro/x.js").await.as_deref(),
            Some("public, max-age=31536000, immutable")
        );
        assert_eq!(cache_header_for("/").await.as_deref(), Some("no-cache"));
        assert_eq!(cache_header_for("/api/v1/apps").await, None);
    }

    #[tokio::test]
    async fn does_not_override_existing_cache_control() {
        use axum::routing::get;
        use tower::ServiceExt;

        // A handler that sets its own Cache-Control must win over the middleware.
        let app = Router::new()
            .route(
                "/_astro/{*rest}",
                get(|| async {
                    (
                        [(axum::http::header::CACHE_CONTROL, "private, max-age=1")],
                        "x",
                    )
                }),
            )
            .layer(axum::middleware::from_fn(dashboard_cache_control));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/_astro/x.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.headers()
                .get(axum::http::header::CACHE_CONTROL)
                .unwrap()
                .to_str()
                .unwrap(),
            "private, max-age=1"
        );
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
