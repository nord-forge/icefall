use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info, warn};

use super::HandlerError;
use crate::context::HandlerContext;

#[derive(Debug, Deserialize)]
struct HealthCheckParams {
    port: u16,
    #[serde(default = "default_retries")]
    retries: u32,
    #[serde(default = "default_interval_ms")]
    interval_ms: u64,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
    path: Option<String>,
}

fn default_retries() -> u32 {
    5
}
fn default_interval_ms() -> u64 {
    2000
}
fn default_timeout_ms() -> u64 {
    5000
}

pub async fn check(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: HealthCheckParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let path = p.path.as_deref().unwrap_or("/");
    let url = format!("http://localhost:{}{path}", p.port);
    let timeout = std::time::Duration::from_millis(p.timeout_ms);
    let interval = std::time::Duration::from_millis(p.interval_ms);

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| HandlerError::Other(format!("failed to build HTTP client: {e}")))?;

    for attempt in 1..=p.retries {
        debug!(attempt, url = %url, "health check attempt");

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                info!(
                    attempt,
                    status = resp.status().as_u16(),
                    "health check passed"
                );
                return Ok(serde_json::json!({
                    "healthy": true,
                    "status": resp.status().as_u16(),
                    "attempts": attempt,
                }));
            }
            Ok(resp) => {
                warn!(
                    attempt,
                    status = resp.status().as_u16(),
                    "health check returned non-success"
                );
            }
            Err(e) => {
                debug!(attempt, error = %e, "health check connection failed");
            }
        }

        if attempt < p.retries {
            tokio::time::sleep(interval).await;
        }
    }

    Err(HandlerError::Other(format!(
        "health check failed after {} attempts on {url}",
        p.retries
    )))
}
