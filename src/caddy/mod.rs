pub mod queue;
pub mod routes;
pub mod types;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaddyError {
    #[error("caddy unreachable: {0}")]
    Unreachable(String),
    #[error("route not found: {0}")]
    RouteNotFound(String),
    #[error("caddy API error: {status} {body}")]
    ApiError { status: u16, body: String },
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

#[derive(Clone)]
pub struct CaddyClient {
    client: reqwest::Client,
    base_url: String,
}

impl CaddyClient {
    pub fn new(admin_url: &str) -> Self {
        // A hung Caddy admin API must not block an async worker forever.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            client,
            base_url: admin_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn health_check(&self) -> Result<(), CaddyError> {
        let url = format!("{}/config/", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| CaddyError::Unreachable(e.to_string()))?;
        Ok(())
    }

    pub(crate) fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }
}
