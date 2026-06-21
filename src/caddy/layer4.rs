//! Layer 4 (raw TCP) proxying via the caddy-l4 module (IF-172). Binds
//! `0.0.0.0:{public_port}` to a container's loopback host port; one server per port.

use crate::caddy::{CaddyClient, CaddyError};

/// Caddy's config key for the layer4 app.
const L4_APP_PATH: &str = "config/apps/layer4";

/// The server name Caddy uses for a given public port. One server per port keeps
/// routes isolated: removing one never touches another.
fn server_name(public_port: u16) -> String {
    format!("l4-{public_port}")
}

impl CaddyClient {
    /// Whether this Caddy build includes the `layer4` app (caddy-l4 plugin).
    /// Probed by validating a minimal config; builds without it reject as "unknown".
    pub async fn has_layer4_module(&self) -> bool {
        let probe = serde_json::json!({
            "apps": { "layer4": { "servers": { "_probe": {
                "listen": ["127.0.0.1:0"],
                "routes": [{
                    "handle": [{ "handler": "proxy", "upstreams": [{ "dial": ["127.0.0.1:1"] }] }]
                }]
            }}}}
        });
        match self.validate_config(&probe).await {
            Ok(()) => true,
            Err(CaddyError::ApiError { body, .. }) => {
                !body.contains("unknown") && !body.contains("not registered")
            }
            // Network/other errors: be conservative and report absent so callers
            // surface a clear "L4 unavailable" error rather than a confusing one.
            Err(_) => false,
        }
    }

    /// Create (or replace) an L4 TCP route exposing `public_port` -> `127.0.0.1:{host_port}`.
    /// Non-empty `allowed_ips` restricts sources; empty = open. Idempotent per `l4-{port}`.
    pub async fn set_tcp_proxy(
        &self,
        public_port: u16,
        host_port: u16,
        allowed_ips: &[String],
    ) -> Result<(), CaddyError> {
        // Build the route. A `remote_ip` matcher restricts sources when an IP
        // whitelist is set; with no matcher the route accepts every connection.
        let mut route = serde_json::json!({
            "handle": [{
                "handler": "proxy",
                "upstreams": [{ "dial": [format!("127.0.0.1:{host_port}")] }]
            }]
        });
        if !allowed_ips.is_empty() {
            route["match"] = serde_json::json!([{
                "remote_ip": { "ranges": allowed_ips }
            }]);
        }

        let server = serde_json::json!({
            "listen": [format!("0.0.0.0:{public_port}")],
            "routes": [route]
        });

        // Ensure the layer4 app shell exists first; a PUT on the leaf server
        // path 404s if `apps/layer4` is absent (idempotent).
        self.ensure_layer4_app().await?;

        let url = format!(
            "{}/{}/servers/{}",
            self.base_url(),
            L4_APP_PATH,
            server_name(public_port)
        );
        let response = self.client().put(&url).json(&server).send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }
        Ok(())
    }

    /// Remove the L4 route for `public_port`. Not-found is treated as success so
    /// disabling/deleting is idempotent even if the route was already gone.
    pub async fn remove_tcp_proxy(&self, public_port: u16) -> Result<(), CaddyError> {
        let url = format!(
            "{}/{}/servers/{}",
            self.base_url(),
            L4_APP_PATH,
            server_name(public_port)
        );
        let response = self.client().delete(&url).send().await?;
        let status = response.status().as_u16();
        if status == 404 || response.status().is_success() {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        Err(CaddyError::ApiError { status, body })
    }

    /// Create the `apps/layer4` shell (`{ "servers": {} }`) if absent, so a later
    /// PUT on a specific server path succeeds. Idempotent.
    async fn ensure_layer4_app(&self) -> Result<(), CaddyError> {
        // GET first — cheap, and avoids a noisy error path on the common case
        // where the app already exists.
        let app_url = format!("{}/{}", self.base_url(), L4_APP_PATH);
        let existing = self.client().get(&app_url).send().await?;
        if existing.status().is_success() {
            // Body is `null` when the key is absent but the parent exists.
            let val: serde_json::Value = existing.json().await.unwrap_or(serde_json::Value::Null);
            if val.is_object() {
                return Ok(());
            }
        }

        // Create the app shell. PUT is idempotent on the app path and creates
        // intermediate keys (`apps`) as needed.
        let shell = serde_json::json!({ "servers": {} });
        let response = self.client().put(&app_url).json(&shell).send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_name_is_port_scoped() {
        assert_eq!(server_name(10000), "l4-10000");
        assert_eq!(server_name(10042), "l4-10042");
    }
}
