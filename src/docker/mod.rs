pub mod containers;
pub mod images;
pub mod logs;
pub mod networks;
pub mod stats;
pub mod volumes;

use bollard::Docker;
use thiserror::Error;

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
}

impl DockerClient {
    pub async fn connect(socket_path: &str) -> Result<Self, DockerError> {
        let docker = Docker::connect_with_socket(socket_path, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| DockerError::Unavailable(e.to_string()))?;

        let client = Self { inner: docker };
        client.ping().await?;
        Ok(client)
    }

    pub async fn connect_default() -> Result<Self, DockerError> {
        let socket = crate::config::defaults::container_socket();
        Self::connect(&socket).await
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
