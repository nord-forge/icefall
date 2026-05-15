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
        let config = NetworkCreateRequest {
            name: name.to_string(),
            driver: Some("bridge".to_string()),
            ..Default::default()
        };

        let response = self.inner().create_network(config).await?;
        Ok(response.id)
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
