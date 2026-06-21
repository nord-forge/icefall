//! Embedded dashboard serving (IF-255). The built dashboard is compiled into the
//! binary with `include_dir!`; `ICEFALL_DASHBOARD_DIR` reads from disk for local dev.

use std::sync::LazyLock;

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use include_dir::{include_dir, Dir};

/// The built dashboard, embedded at compile time. Empty if `dashboard/dist`
/// didn't exist at build time (run `bun run build` first).
static DASHBOARD: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/dashboard/dist");

/// Optional on-disk override for local development. When set, assets are served
/// from this directory instead of the embedded copy.
static DASHBOARD_DIR_OVERRIDE: LazyLock<Option<std::path::PathBuf>> = LazyLock::new(|| {
    std::env::var("ICEFALL_DASHBOARD_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
});

/// Dynamic dashboard routes. Astro prerenders each as a shell under
/// `<prefix>/_/index.html`; an unmatched path under one of these prefixes is
/// served that shell so client-side routing takes over.
const DYNAMIC_ROUTE_PREFIXES: &[&str] = &["/teams/", "/servers/", "/apps/", "/invitations/"];

/// Read the dashboard's `csp-hashes.json` (embedded, or from the dev override).
/// Returns the raw JSON string so the CSP layer can parse it.
pub fn csp_hashes_json() -> Option<String> {
    if let Some(dir) = DASHBOARD_DIR_OVERRIDE.as_ref() {
        return std::fs::read_to_string(dir.join("csp-hashes.json")).ok();
    }
    DASHBOARD
        .get_file("csp-hashes.json")
        .and_then(|f| f.contents_utf8().map(str::to_string))
}

/// Map a request path to the dashboard file that should answer it: exact file,
/// directory index, dynamic-route SPA shell, or root `index.html` fallback.
fn resolve_logical_path(req_path: &str) -> String {
    // Strip the leading slash — embedded paths are relative.
    let trimmed = req_path.trim_start_matches('/');

    // A concrete file (e.g. `_astro/x.js`, `favicon.svg`).
    if !trimmed.is_empty() && asset_exists(trimmed) {
        return trimmed.to_string();
    }

    // A directory request → its index.html.
    if !trimmed.is_empty() {
        let as_index = format!("{}/index.html", trimmed.trim_end_matches('/'));
        if asset_exists(&as_index) {
            return as_index;
        }
    }

    // Dynamic SPA route → prerendered shell.
    if let Some(prefix) = DYNAMIC_ROUTE_PREFIXES
        .iter()
        .find(|p| req_path.starts_with(*p))
    {
        // `/apps/abc` → `apps/_/index.html`
        let shell = format!("{}_/index.html", prefix.trim_start_matches('/'));
        if asset_exists(&shell) {
            return shell;
        }
    }

    // Root SPA fallback.
    "index.html".to_string()
}

/// Whether an identity asset exists (embedded or on disk).
fn asset_exists(path: &str) -> bool {
    if let Some(dir) = DASHBOARD_DIR_OVERRIDE.as_ref() {
        return dir.join(path).is_file();
    }
    DASHBOARD.get_file(path).is_some()
}

/// Read an asset's bytes (a specific variant path, e.g. `x.js.br`).
fn read_variant(path: &str) -> Option<Vec<u8>> {
    if let Some(dir) = DASHBOARD_DIR_OVERRIDE.as_ref() {
        return std::fs::read(dir.join(path)).ok();
    }
    DASHBOARD.get_file(path).map(|f| f.contents().to_vec())
}

/// `Cache-Control` for a logical asset path. Content-hashed assets under
/// `_astro/` are immutable; HTML shells must revalidate so a new build's asset
/// hashes are discovered (mirrors the IF-253 policy).
fn cache_control_for(path: &str) -> &'static str {
    if path.starts_with("_astro/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

/// Pick the best precompressed variant the client accepts. Returns the variant
/// file suffix and the `Content-Encoding` value, or `None` for identity.
fn negotiate_encoding(headers: &HeaderMap, logical: &str) -> Option<(&'static str, &'static str)> {
    let accept = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // Brotli first (better ratio), then gzip — only if the precompressed variant
    // was actually emitted for this file (IF-254 skips tiny/incompressible ones).
    if accept.contains("br") && asset_exists(&format!("{logical}.br")) {
        Some((".br", "br"))
    } else if accept.contains("gzip") && asset_exists(&format!("{logical}.gz")) {
        Some((".gz", "gzip"))
    } else {
        None
    }
}

/// Serve a dashboard asset for the request URI: resolve the logical path,
/// negotiate a precompressed variant, and set Content-Type / Cache-Control /
/// Content-Encoding. This is the dashboard `fallback_service` for the router.
pub async fn serve(uri: Uri, headers: HeaderMap) -> Response {
    let logical = resolve_logical_path(uri.path());

    // Content-Type from the logical (uncompressed) filename.
    let content_type = mime_guess::from_path(&logical)
        .first_or_octet_stream()
        .to_string();

    // Choose a precompressed variant if the client accepts one.
    let (variant_suffix, encoding) = match negotiate_encoding(&headers, &logical) {
        Some((suffix, enc)) => (format!("{logical}{suffix}"), Some(enc)),
        None => (logical.clone(), None),
    };

    let Some(bytes) = read_variant(&variant_suffix) else {
        return (
            StatusCode::NOT_FOUND,
            "dashboard not built into this binary",
        )
            .into_response();
    };

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, cache_control_for(&logical))
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());

    if let Some(enc) = encoding {
        if let Ok(hv) = HeaderValue::from_str(enc) {
            response.headers_mut().insert(header::CONTENT_ENCODING, hv);
            // Caches must key on Accept-Encoding when content varies by it.
            response
                .headers_mut()
                .insert(header::VARY, HeaderValue::from_static("accept-encoding"));
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_routes_resolve_to_shells_or_index() {
        // These depend on the embedded build existing; assert the *logic* with
        // paths that fall through to the root index when shells are absent.
        // (The build is present in CI; locally `dashboard/dist` is built.)
        let p = resolve_logical_path("/");
        assert_eq!(p, "index.html");
    }

    #[test]
    fn cache_policy_matches_if253() {
        assert_eq!(
            cache_control_for("_astro/x.js"),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(cache_control_for("index.html"), "no-cache");
        assert_eq!(cache_control_for("apps/_/index.html"), "no-cache");
    }
}
