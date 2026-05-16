pub mod containers;
pub mod images;
pub mod logs;
pub mod networks;
pub mod quirks;
pub mod stats;
pub mod volumes;

#[cfg(test)]
mod runtime_compat_tests;

use bollard::Docker;
use thiserror::Error;

pub use quirks::{DnsBackend, RuntimeQuirks};

use crate::config::ContainerRuntime;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("container not found: {0}")]
    ContainerNotFound(String),
    #[error("image not found: {0}")]
    ImageNotFound(String),
    #[error("docker engine unavailable: {0}")]
    Unavailable(String),
    #[error("docker API error: {0}")]
    Api(#[from] bollard::errors::Error),
    #[error("image build failed: {0}")]
    BuildFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct DockerClient {
    inner: Docker,
    quirks: RuntimeQuirks,
}

impl DockerClient {
    pub async fn connect(socket_path: &str) -> Result<Self, DockerError> {
        let docker = Docker::connect_with_socket(socket_path, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| DockerError::Unavailable(e.to_string()))?;

        // Provisional client so ping/info can run; quirks filled in below.
        let mut client = Self {
            inner: docker,
            quirks: RuntimeQuirks::docker_default(),
        };
        client.ping().await?;
        client.quirks = client.detect_quirks(socket_path).await;

        if client.quirks.runtime == ContainerRuntime::Podman {
            tracing::info!(
                rootless = client.quirks.rootless,
                "Container runtime is Podman; rootless={}",
                client.quirks.rootless
            );
        }
        Ok(client)
    }

    pub async fn connect_default() -> Result<Self, DockerError> {
        let socket = crate::config::defaults::container_socket();
        Self::connect(&socket).await
    }

    /// Quirks of the connected runtime (Docker vs Podman, rootless, etc).
    pub fn quirks(&self) -> &RuntimeQuirks {
        &self.quirks
    }

    /// Detect runtime quirks from `info` and the socket path. Falls back to the
    /// Docker baseline if introspection fails — never blocks connecting.
    async fn detect_quirks(&self, socket_path: &str) -> RuntimeQuirks {
        let runtime = match self.runtime_version().await {
            Ok(info) if info.name == "podman" => ContainerRuntime::Podman,
            _ => ContainerRuntime::Docker,
        };

        if runtime == ContainerRuntime::Docker {
            return RuntimeQuirks::docker_default();
        }

        let security_options = self
            .inner
            .info()
            .await
            .ok()
            .and_then(|i| i.security_options)
            .unwrap_or_default();

        RuntimeQuirks::detect(runtime, socket_path, &security_options)
    }

    pub async fn ping(&self) -> Result<(), DockerError> {
        self.inner
            .ping()
            .await
            .map_err(|e| DockerError::Unavailable(format!("Container runtime ping failed: {e}")))?;
        Ok(())
    }

    pub async fn runtime_version(&self) -> Result<RuntimeInfo, DockerError> {
        let version = self.inner.version().await?;
        let server_version = version.version.unwrap_or_default();
        let api_version = version.api_version.unwrap_or_default();
        let os = version.os.unwrap_or_default();
        let arch = version.arch.unwrap_or_default();

        let runtime_name = version
            .components
            .as_ref()
            .and_then(|c| {
                c.iter()
                    .find(|comp| comp.name.to_lowercase().contains("podman"))
            })
            .map_or_else(|| "docker".to_string(), |_| "podman".to_string());

        Ok(RuntimeInfo {
            name: runtime_name,
            version: server_version,
            api_version,
            os,
            arch,
        })
    }

    pub(crate) fn inner(&self) -> &Docker {
        &self.inner
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeInfo {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub os: String,
    pub arch: String,
}
