use serde::{Deserialize, Serialize};

/// A snapshot of an app's full Caddy proxy config, captured before any change so
/// a broken edit can be rolled back. Kept to the last 10 per app (see
/// `record_proxy_config_history`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxyConfigHistory {
    pub id: String,
    pub app_id: String,
    pub config: String,
    pub created_at: String,
}

/// Global proxy defaults (single row, id = "global"). Applied across every app's
/// auto-generated routes.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxySettings {
    pub id: String,
    /// JSON object of header name -> value, or NULL.
    pub default_headers: Option<String>,
    /// JSON: { enabled, requests, window, burst }, or NULL.
    pub default_rate_limit: Option<String>,
    pub force_https: bool,
    pub updated_at: String,
}

/// Patch for `proxy_settings`. Each `Some` overwrites; `None` leaves the column
/// as-is. The inner `Option<String>` distinguishes "set to NULL" from "unset".
#[derive(Debug, Default, Deserialize)]
pub struct UpdateProxySettings {
    pub default_headers: Option<Option<String>>,
    pub default_rate_limit: Option<Option<String>>,
    pub force_https: Option<bool>,
}
