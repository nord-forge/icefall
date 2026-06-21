//! Proxy-management extensions to the Caddy client (IF-149): config read/load,
//! module detection, and translating middleware presets into Caddy handlers.

use serde::{Deserialize, Serialize};

use crate::caddy::{CaddyClient, CaddyError};

/// Middleware presets stored per app. Serialized to the apps.proxy_presets JSON
/// column. Every field is optional so the UI can toggle one preset at a time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyPresets {
    #[serde(default)]
    pub force_https: Option<bool>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitPreset>,
    #[serde(default)]
    pub basic_auth: Option<BasicAuthPreset>,
    #[serde(default)]
    pub redirects: Vec<RedirectRule>,
    #[serde(default)]
    pub headers: Vec<HeaderRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitPreset {
    pub enabled: bool,
    /// Allowed requests within the window.
    pub requests: u32,
    /// "second" or "minute".
    #[serde(default = "default_window")]
    pub window: String,
    #[serde(default)]
    pub burst: u32,
    /// true = per client IP, false = global across the app.
    #[serde(default = "default_true")]
    pub per_ip: bool,
}

fn default_window() -> String {
    "minute".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAuthPreset {
    pub enabled: bool,
    pub username: String,
    /// bcrypt hash of the password — never the plaintext. Populated server-side
    /// from the transient `password` field.
    #[serde(default)]
    pub password_hash: String,
    /// Plaintext password from the client on update only; hashed then cleared.
    /// Empty means "keep the existing password".
    #[serde(default, skip_serializing)]
    pub password: Option<String>,
    /// Optional path prefix the auth applies to; None = whole app.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectRule {
    pub from: String,
    pub to: String,
    /// 301 or 302.
    #[serde(default = "default_redirect_status")]
    pub status: u16,
}

fn default_redirect_status() -> u16 {
    301
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderRule {
    pub name: String,
    pub value: String,
}

impl CaddyClient {
    /// The complete Caddy config (`GET /config/`). Returns an empty object if
    /// Caddy has no config loaded yet.
    pub async fn get_full_config(&self) -> Result<serde_json::Value, CaddyError> {
        let url = format!("{}/config/", self.base_url());
        let response = self.client().get(&url).send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }
        Ok(response.json().await.unwrap_or(serde_json::Value::Null))
    }

    /// Apply custom routes for one app, scoped to its domains: removes its
    /// host-matched routes and appends `routes` (a route object or array), leaving others.
    pub async fn apply_scoped_routes(
        &self,
        domains: &[String],
        routes: &serde_json::Value,
    ) -> Result<(), CaddyError> {
        // Normalize the input to a list of route objects.
        let new_routes: Vec<serde_json::Value> = match routes {
            serde_json::Value::Array(arr) => arr.clone(),
            serde_json::Value::Object(_) => vec![routes.clone()],
            _ => {
                return Err(CaddyError::ApiError {
                    status: 400,
                    body: "custom proxy config must be a route object or array of routes".into(),
                })
            }
        };

        // Remove this app's current routes first so we don't duplicate. Ignore
        // not-found — a fresh app may have none yet.
        for domain in domains {
            match self.remove_route(domain).await {
                Ok(()) | Err(CaddyError::RouteNotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }

        let url = format!("{}/config/apps/http/servers/srv0/routes", self.base_url());
        for route in &new_routes {
            let response = self.client().post(&url).json(route).send().await?;
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                return Err(CaddyError::ApiError { status, body });
            }
        }
        Ok(())
    }

    /// All routes on srv0 as raw JSON, for the read-only viewer.
    pub async fn get_routes_config(&self) -> Result<serde_json::Value, CaddyError> {
        let url = format!("{}/config/apps/http/servers/srv0/routes", self.base_url());
        let response = self.client().get(&url).send().await?;
        if response.status().as_u16() == 404 {
            return Ok(serde_json::Value::Array(vec![]));
        }
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }
        Ok(response.json().await.unwrap_or(serde_json::Value::Null))
    }

    /// Validate a full Caddy config without applying it (`POST /load` with the
    /// `Caddy-Validate-Only` header). On invalid config Caddy returns 400 with a
    /// descriptive body, surfaced as the error string.
    pub async fn validate_config(&self, config: &serde_json::Value) -> Result<(), CaddyError> {
        let url = format!("{}/load", self.base_url());
        let response = self
            .client()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Caddy-Validate-Only", "true")
            .json(config)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }
        Ok(())
    }

    /// Apply a full Caddy config (`POST /load`). Caddy validates atomically and
    /// keeps the previous config if the new one is rejected.
    pub async fn load_config(&self, config: &serde_json::Value) -> Result<(), CaddyError> {
        let url = format!("{}/load", self.base_url());
        let response = self
            .client()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(config)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }
        Ok(())
    }

    /// Whether this Caddy build includes the `http.handlers.rate_limit` module.
    /// Probed by validating a tiny config; callers pick the module or 429 fallback.
    pub async fn has_rate_limit_module(&self) -> bool {
        // The most reliable probe is validating a tiny config that uses the module.
        let probe = serde_json::json!({
            "apps": { "http": { "servers": { "_probe": {
                "listen": [":0"],
                "routes": [{ "handle": [{ "handler": "rate_limit" }] }]
            }}}}
        });
        // If validation fails specifically because the module is unknown, it's absent.
        match self.validate_config(&probe).await {
            Ok(()) => true,
            Err(CaddyError::ApiError { body, .. }) => {
                !body.contains("unknown") && !body.contains("not registered")
            }
            // Network/other errors: be conservative and use the fallback.
            Err(_) => false,
        }
    }
}

/// Build the ordered Caddy handler objects for an app's presets (wrapped by the
/// caller alongside reverse_proxy). `has_rate_limit_module` picks module vs 429 fallback.
pub fn presets_to_handlers(
    presets: &ProxyPresets,
    has_rate_limit_module: bool,
) -> Vec<serde_json::Value> {
    let mut handlers = Vec::new();

    // Rate limiting first so it can short-circuit before auth/proxy work.
    if let Some(rl) = presets.rate_limit.as_ref().filter(|r| r.enabled) {
        handlers.push(rate_limit_handler(rl, has_rate_limit_module));
    }

    // Basic auth before the proxy so unauthenticated requests never reach upstream.
    if let Some(ba) = presets.basic_auth.as_ref().filter(|b| b.enabled) {
        handlers.push(serde_json::json!({
            "handler": "authentication",
            "providers": {
                "http_basic": {
                    "accounts": [{ "username": ba.username, "password": ba.password_hash }]
                }
            }
        }));
    }

    // Custom response headers.
    if !presets.headers.is_empty() {
        let mut set = serde_json::Map::new();
        for h in &presets.headers {
            set.insert(h.name.clone(), serde_json::json!([h.value]));
        }
        handlers.push(serde_json::json!({
            "handler": "headers",
            "response": { "set": set }
        }));
    }

    // Static redirects via the static_response handler.
    for r in &presets.redirects {
        handlers.push(serde_json::json!({
            "handler": "static_response",
            "status_code": r.status,
            "headers": { "Location": [r.to.clone()] },
        }));
    }

    handlers
}

fn rate_limit_handler(rl: &RateLimitPreset, has_module: bool) -> serde_json::Value {
    let window = if rl.window == "second" { "1s" } else { "1m" };
    if has_module {
        // Native caddy-ratelimit module.
        let key = if rl.per_ip {
            "{http.request.remote.host}"
        } else {
            "static"
        };
        serde_json::json!({
            "handler": "rate_limit",
            "rate_limits": {
                "app": {
                    "key": key,
                    "window": window,
                    "max_events": rl.requests + rl.burst,
                }
            }
        })
    } else {
        // Fallback: respond 429. Without the plugin Caddy can't count requests;
        // this documents intent and blocks. Real enforcement requires the plugin.
        serde_json::json!({
            "handler": "static_response",
            "status_code": 429,
            "headers": {
                "Retry-After": [window],
                "X-RateLimit-Fallback": ["caddy-ratelimit module not installed"]
            },
            "body": "Rate limit configured but caddy-ratelimit module is not installed."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_presets_produce_no_handlers() {
        let handlers = presets_to_handlers(&ProxyPresets::default(), true);
        assert!(handlers.is_empty());
    }

    #[test]
    fn rate_limit_uses_native_module_when_present() {
        let presets = ProxyPresets {
            rate_limit: Some(RateLimitPreset {
                enabled: true,
                requests: 100,
                window: "minute".into(),
                burst: 10,
                per_ip: true,
            }),
            ..Default::default()
        };
        let handlers = presets_to_handlers(&presets, true);
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0]["handler"], "rate_limit");
        assert_eq!(handlers[0]["rate_limits"]["app"]["max_events"], 110);
    }

    #[test]
    fn rate_limit_falls_back_to_429_without_module() {
        let presets = ProxyPresets {
            rate_limit: Some(RateLimitPreset {
                enabled: true,
                requests: 100,
                window: "second".into(),
                burst: 0,
                per_ip: false,
            }),
            ..Default::default()
        };
        let handlers = presets_to_handlers(&presets, false);
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0]["handler"], "static_response");
        assert_eq!(handlers[0]["status_code"], 429);
    }

    #[test]
    fn disabled_presets_are_skipped() {
        let presets = ProxyPresets {
            rate_limit: Some(RateLimitPreset {
                enabled: false,
                requests: 100,
                window: "minute".into(),
                burst: 0,
                per_ip: true,
            }),
            basic_auth: Some(BasicAuthPreset {
                enabled: false,
                username: "u".into(),
                password_hash: "h".into(),
                password: None,
                path: None,
            }),
            ..Default::default()
        };
        assert!(presets_to_handlers(&presets, true).is_empty());
    }

    #[test]
    fn auth_and_headers_and_redirects_combine_in_order() {
        let presets = ProxyPresets {
            basic_auth: Some(BasicAuthPreset {
                enabled: true,
                username: "u".into(),
                password_hash: "$2b$hash".into(),
                password: None,
                path: None,
            }),
            headers: vec![HeaderRule {
                name: "X-Frame-Options".into(),
                value: "DENY".into(),
            }],
            redirects: vec![RedirectRule {
                from: "/old".into(),
                to: "https://example.com/new".into(),
                status: 302,
            }],
            ..Default::default()
        };
        let handlers = presets_to_handlers(&presets, true);
        assert_eq!(handlers.len(), 3);
        assert_eq!(handlers[0]["handler"], "authentication");
        assert_eq!(handlers[1]["handler"], "headers");
        assert_eq!(handlers[2]["handler"], "static_response");
        assert_eq!(handlers[2]["status_code"], 302);
    }
}
