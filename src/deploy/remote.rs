use std::sync::Arc;
use std::time::Duration;

use icefall_common::protocol::AgentMessage;
use tracing::debug;

use crate::agent::registry::AgentRegistry;
use crate::deploy::DeployError;

pub struct RemoteExecutor {
    registry: Arc<AgentRegistry>,
    pub server_id: String,
    pub server_host: String,
    pub server_public_key: Option<String>,
}

impl RemoteExecutor {
    pub fn new(
        registry: Arc<AgentRegistry>,
        server_id: String,
        server_host: String,
        server_public_key: Option<String>,
    ) -> Self {
        Self {
            registry,
            server_id,
            server_host,
            server_public_key,
        }
    }

    async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, DeployError> {
        debug!(server = %self.server_id, method, "remote call");
        let response = self
            .registry
            .send_request(&self.server_id, method.to_string(), params)
            .await
            .map_err(|e| {
                if e.contains("not connected") {
                    DeployError::AgentOffline(e)
                } else if e.contains("timed out") {
                    DeployError::AgentTimeout(e)
                } else {
                    DeployError::RemoteOp(e)
                }
            })?;

        match response {
            AgentMessage::Response {
                result: Some(val),
                error: None,
                ..
            } => Ok(val),
            AgentMessage::Response {
                error: Some(err), ..
            } => Err(DeployError::RemoteOp(err)),
            _ => Err(DeployError::RemoteOp("unexpected response".to_string())),
        }
    }

    async fn call_with_timeout(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, DeployError> {
        debug!(server = %self.server_id, method, timeout_secs = timeout.as_secs(), "remote call (custom timeout)");
        let response = self
            .registry
            .send_request_with_timeout(&self.server_id, method.to_string(), params, timeout)
            .await
            .map_err(|e| {
                if e.contains("not connected") {
                    DeployError::AgentOffline(e)
                } else if e.contains("timed out") {
                    DeployError::AgentTimeout(e)
                } else {
                    DeployError::RemoteOp(e)
                }
            })?;

        match response {
            AgentMessage::Response {
                result: Some(val),
                error: None,
                ..
            } => Ok(val),
            AgentMessage::Response {
                error: Some(err), ..
            } => Err(DeployError::RemoteBuild(err)),
            _ => Err(DeployError::RemoteOp("unexpected response".to_string())),
        }
    }

    // --- Build ---

    #[allow(clippy::too_many_arguments)]
    pub async fn run_build(
        &self,
        repo_url: &str,
        branch: &str,
        deploy_id: &str,
        app_name: &str,
        env_vars: &[String],
        config: Option<&serde_json::Value>,
        timeout: Duration,
    ) -> Result<String, DeployError> {
        let params = serde_json::json!({
            "repo_url": repo_url,
            "branch": branch,
            "deploy_id": deploy_id,
            "app_name": app_name,
            "env_vars": env_vars,
            "config": config,
        });

        let result = self.call_with_timeout("build.run", params, timeout).await?;
        result["image_tag"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| DeployError::RemoteBuild("no image_tag in response".to_string()))
    }

    // --- Container operations ---

    pub async fn create_network(&self, name: &str) -> Result<(), DeployError> {
        let _ = self
            .call("network.create", serde_json::json!({ "name": name }))
            .await;
        Ok(())
    }

    pub async fn create_container(&self, params: serde_json::Value) -> Result<String, DeployError> {
        let result = self.call("container.create", params).await?;
        result["id"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| DeployError::ContainerCreate("no container id in response".to_string()))
    }

    pub async fn start_container(&self, id: &str) -> Result<(), DeployError> {
        self.call("container.start", serde_json::json!({ "id": id }))
            .await?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str, timeout_secs: i64) -> Result<(), DeployError> {
        self.call(
            "container.stop",
            serde_json::json!({ "id": id, "timeout_secs": timeout_secs }),
        )
        .await?;
        Ok(())
    }

    pub async fn remove_container(&self, id: &str) -> Result<(), DeployError> {
        self.call("container.remove", serde_json::json!({ "id": id }))
            .await?;
        Ok(())
    }

    pub async fn inspect_container(&self, id: &str) -> Result<serde_json::Value, DeployError> {
        self.call("container.inspect", serde_json::json!({ "id": id }))
            .await
    }

    pub async fn exec_in_container(&self, id: &str, cmd: &[String]) -> Result<String, DeployError> {
        let result = self
            .call(
                "container.exec",
                serde_json::json!({ "id": id, "cmd": cmd }),
            )
            .await?;
        Ok(result
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    pub async fn list_containers_by_label(
        &self,
        _label: &str,
    ) -> Result<Vec<serde_json::Value>, DeployError> {
        let result = self.call("container.list", serde_json::json!({})).await?;
        let containers = result["containers"].as_array().cloned().unwrap_or_default();
        Ok(containers)
    }

    // --- Health check ---

    pub async fn health_check(
        &self,
        port: u16,
        retries: u32,
        interval_ms: u64,
    ) -> Result<(), DeployError> {
        self.call(
            "health.check",
            serde_json::json!({
                "port": port,
                "retries": retries,
                "interval_ms": interval_ms,
            }),
        )
        .await?;
        Ok(())
    }

    // --- Caddy routes ---

    pub async fn add_caddy_route(&self, domain: &str, upstream: &str) -> Result<(), DeployError> {
        self.call(
            "caddy.add_route",
            serde_json::json!({ "domain": domain, "upstream": upstream }),
        )
        .await?;
        Ok(())
    }

    pub async fn update_caddy_route(
        &self,
        domain: &str,
        upstream: &str,
    ) -> Result<(), DeployError> {
        self.call(
            "caddy.update_route",
            serde_json::json!({ "domain": domain, "upstream": upstream }),
        )
        .await?;
        Ok(())
    }

    pub async fn remove_caddy_route(&self, domain: &str) -> Result<(), DeployError> {
        self.call(
            "caddy.remove_route",
            serde_json::json!({ "domain": domain }),
        )
        .await?;
        Ok(())
    }
}
