pub mod compose;
pub mod drift;
pub mod envelope;
pub mod health;
pub mod manager;
pub mod native;
pub mod preview;
pub mod remote;
pub mod s3_mount;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployTarget {
    Local,
    Remote { server_id: String },
}

#[derive(Debug, Error)]
pub enum DeployError {
    #[error("container creation failed: {0}")]
    ContainerCreate(String),
    #[error("health check failed after {0} attempts")]
    HealthCheckFailed(u32),
    #[error("caddy route update failed: {0}")]
    RouteUpdate(String),
    #[error("rollback failed: {0}")]
    RollbackFailed(String),
    #[error("compose file parse error: {0}")]
    ComposeParseError(String),
    #[error("native build failed: {0}")]
    NativeBuild(String),
    #[error("docker error: {0}")]
    Docker(#[from] crate::docker::DockerError),
    #[error("caddy error: {0}")]
    Caddy(#[from] crate::caddy::CaddyError),
    #[error("database error: {0}")]
    Database(#[from] crate::db::DbError),
    #[error("agent offline: {0}")]
    AgentOffline(String),
    #[error("agent command timed out: {0}")]
    AgentTimeout(String),
    #[error("remote build failed: {0}")]
    RemoteBuild(String),
    #[error("remote operation failed: {0}")]
    RemoteOp(String),
    #[error("envelope encryption failed: {0}")]
    EnvelopeEncrypt(String),
}

/// Retry a fallible deploy-state write up to three times, logging each failure.
/// Used for DB updates whose loss would desync the recorded deploy state from
/// reality (breaking rollbacks and health monitoring) — never swallow them.
pub async fn retry_state_write<F, Fut, T, E>(what: &str, mut op: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_err = None;
    for attempt in 1..=3u32 {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                tracing::warn!(operation = what, attempt, error = %e, "deploy-state write failed");
                last_err = Some(e);
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(200 * attempt as u64))
                        .await;
                }
            }
        }
    }
    Err(last_err.expect("loop runs at least once"))
}
