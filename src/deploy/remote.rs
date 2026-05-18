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

    // --- Image transfer ---

    /// Transfer a built image to the remote server and load it into the remote
    /// Docker daemon.
    ///
    /// The raw `docker save` tar (`image_tar`) is gzip-compressed, split into
    /// `chunk_size`-byte chunks, and streamed as binary WebSocket frames. Each
    /// chunk carries a SHA-256 the agent verifies; a failed chunk is retried
    /// individually. The whole transfer is retried up to `MAX_TRANSFER_ATTEMPTS`
    /// times on a fatal error.
    pub async fn load_image(
        &self,
        image_tar: &[u8],
        chunk_size: usize,
        timeout: Duration,
    ) -> Result<(), DeployError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Compress the tar once; the compressed payload is what gets chunked.
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(image_tar)
            .map_err(|e| DeployError::RemoteOp(format!("gzip: {e}")))?;
        let compressed = encoder
            .finish()
            .map_err(|e| DeployError::RemoteOp(format!("gzip finish: {e}")))?;

        tracing::info!(
            server = %self.server_id,
            raw_bytes = image_tar.len(),
            compressed_bytes = compressed.len(),
            "transferring image to remote server"
        );

        const MAX_TRANSFER_ATTEMPTS: u32 = 3;
        let mut last_err = None;
        for attempt in 1..=MAX_TRANSFER_ATTEMPTS {
            match self
                .transfer_compressed_image(&compressed, chunk_size, timeout)
                .await
            {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        server = %self.server_id,
                        attempt,
                        "image transfer attempt failed: {e}"
                    );
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| DeployError::RemoteOp("image transfer failed".to_string())))
    }

    /// Perform one full transfer attempt of the already-compressed payload.
    async fn transfer_compressed_image(
        &self,
        compressed: &[u8],
        chunk_size: usize,
        timeout: Duration,
    ) -> Result<(), DeployError> {
        use icefall_common::transfer::{hex_encode, sha256_hex, sha256_of, ChunkFrame};

        let chunk_size = chunk_size.max(64 * 1024);
        let chunks: Vec<&[u8]> = compressed.chunks(chunk_size).collect();
        let total_chunks = chunks.len() as u32;

        // Fresh random transfer id per attempt.
        let transfer_id: [u8; 16] = {
            use rand::RngExt;
            rand::rng().random()
        };
        let transfer_hex = hex_encode(&transfer_id);
        let total_sha256 = sha256_hex(&sha256_of(compressed));

        // begin
        self.call_with_timeout(
            "image.load.begin",
            serde_json::json!({
                "transfer_id": transfer_hex,
                "total_chunks": total_chunks,
                "chunk_size": chunk_size,
                "total_sha256": total_sha256,
            }),
            timeout,
        )
        .await?;

        // chunks, with per-chunk retry
        const MAX_CHUNK_ATTEMPTS: u32 = 4;
        for (index, payload) in chunks.iter().enumerate() {
            let frame = ChunkFrame {
                transfer_id,
                chunk_index: index as u32,
                total_chunks,
                sha256: sha256_of(payload),
                payload: payload.to_vec(),
            };
            let encoded = frame.encode();

            let mut chunk_ok = false;
            let mut chunk_err = None;
            for chunk_attempt in 1..=MAX_CHUNK_ATTEMPTS {
                match self
                    .registry
                    .send_chunk(
                        &self.server_id,
                        &transfer_hex,
                        index as u32,
                        encoded.clone(),
                        timeout,
                    )
                    .await
                {
                    Ok(ack) if ack.ok => {
                        chunk_ok = true;
                        break;
                    }
                    Ok(ack) => {
                        chunk_err = ack.error;
                        tracing::warn!(
                            "chunk {index} rejected (attempt {chunk_attempt}): {chunk_err:?}"
                        );
                    }
                    Err(e) => {
                        chunk_err = Some(e);
                        tracing::warn!(
                            "chunk {index} send failed (attempt {chunk_attempt}): {chunk_err:?}"
                        );
                    }
                }
            }
            if !chunk_ok {
                return Err(DeployError::RemoteOp(format!(
                    "chunk {index} failed after {MAX_CHUNK_ATTEMPTS} attempts: {}",
                    chunk_err.unwrap_or_else(|| "unknown error".to_string())
                )));
            }
        }

        // commit
        self.call_with_timeout(
            "image.load.commit",
            serde_json::json!({ "transfer_id": transfer_hex }),
            timeout,
        )
        .await?;

        Ok(())
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
