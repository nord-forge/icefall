use bollard::models::{
    EndpointSettings, NetworkConnectRequest, NetworkCreateRequest, NetworkDisconnectRequest,
};
use bollard::query_parameters::ListNetworksOptionsBuilder;

use crate::docker::{DockerClient, DockerError};

#[derive(Debug, Clone, serde::Serialize)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
}

impl DockerClient {
    pub async fn list_networks(&self) -> Result<Vec<NetworkInfo>, DockerError> {
        let options = ListNetworksOptionsBuilder::default().build();
        let networks = self.inner().list_networks(Some(options)).await?;
        Ok(networks
            .into_iter()
            .map(|n| NetworkInfo {
                id: n.id.unwrap_or_default(),
                name: n.name.unwrap_or_default(),
                driver: n.driver.unwrap_or_default(),
            })
            .collect())
    }

    pub async fn create_network(&self, name: &str) -> Result<String, DockerError> {
        // The `bridge` driver provides container DNS on both runtimes: Docker
        // has built-in DNS on user networks, and Podman 4+ uses netavark +
        // aardvark-dns. No explicit DNS flag exists in the Docker API.
        let config = NetworkCreateRequest {
            name: name.to_string(),
            driver: Some("bridge".to_string()),
            ..Default::default()
        };

        let response = self.inner().create_network(config).await?;

        if let Some(ref warning) = non_empty(&response.warning) {
            tracing::warn!(network = name, "network create warning: {warning}");
        }
        Ok(response.id)
    }

    /// Verify container-to-container DNS works by creating a throwaway network,
    /// inspecting its driver, and removing it. Logs a clear remediation hint if
    /// the runtime cannot resolve container names (e.g. Podman without
    /// `aardvark-dns`). Best-effort: never fails the caller.
    pub async fn check_network_dns(&self) {
        use crate::config::ContainerRuntime;

        // Docker always has built-in DNS on user networks — nothing to check.
        if self.quirks().runtime == ContainerRuntime::Docker {
            return;
        }

        let probe_name = "icefall-dns-probe";
        let _ = self.remove_network(probe_name).await; // clear any stale probe

        match self.create_network(probe_name).await {
            Ok(_) => {
                let driver_ok = self
                    .list_networks()
                    .await
                    .ok()
                    .and_then(|nets| nets.into_iter().find(|n| n.name == probe_name))
                    .map(|n| n.driver == "bridge")
                    .unwrap_or(false);
                let _ = self.remove_network(probe_name).await;

                if driver_ok {
                    tracing::info!("Podman container DNS available (netavark bridge)");
                } else {
                    tracing::warn!(
                        "Podman network DNS may be unavailable — install `aardvark-dns` \
                         so containers can resolve each other by name. Load balancing is \
                         unaffected (Caddy routes by host:port)."
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Could not verify Podman network DNS ({e}); if app networking \
                     misbehaves, ensure `netavark` and `aardvark-dns` are installed"
                );
            }
        }
    }

    pub async fn remove_network(&self, name: &str) -> Result<(), DockerError> {
        self.inner().remove_network(name).await?;
        Ok(())
    }

    pub async fn connect_container_to_network(
        &self,
        network: &str,
        container: &str,
    ) -> Result<(), DockerError> {
        let config = NetworkConnectRequest {
            container: container.to_string(),
            endpoint_config: Some(EndpointSettings::default()),
        };
        self.inner().connect_network(network, config).await?;
        Ok(())
    }

    pub async fn connect_container_to_network_with_alias(
        &self,
        network: &str,
        container: &str,
        alias: &str,
    ) -> Result<(), DockerError> {
        let config = NetworkConnectRequest {
            container: container.to_string(),
            endpoint_config: Some(EndpointSettings {
                aliases: Some(vec![alias.to_string()]),
                ..Default::default()
            }),
        };
        self.inner().connect_network(network, config).await?;
        Ok(())
    }

    pub async fn disconnect_container_from_network(
        &self,
        network: &str,
        container: &str,
    ) -> Result<(), DockerError> {
        self.inner()
            .disconnect_network(
                network,
                NetworkDisconnectRequest {
                    container: container.to_string(),
                    force: Some(false),
                },
            )
            .await?;
        Ok(())
    }
}

/// Return the string if it is non-empty, else `None`.
fn non_empty(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
