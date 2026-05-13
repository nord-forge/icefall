mod helpers;
mod interpolation;
pub mod types;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

pub use types::{
    ComposeCommand, ComposeDeployResult, ComposeEnvironment, ComposeFile, ComposeService,
    ComposeServiceResult, ComposeDependsOn, deserialize_string_or_number_vec,
};

use crate::db::models::App;
use crate::db::Database;
use crate::deploy::DeployError;
use crate::docker::containers::ContainerConfig;
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

use helpers::{
    parse_image_ref, resolve_restart_policy, resolve_service_command, resolve_service_env,
    resolve_service_ports, resolve_service_volumes,
};

pub struct ComposeDeployer {
    docker: Arc<DockerClient>,
    db: Arc<dyn Database>,
    event_bus: Arc<EventBus>,
}

impl ComposeDeployer {
    pub fn new(docker: Arc<DockerClient>, db: Arc<dyn Database>, event_bus: Arc<EventBus>) -> Self {
        Self {
            docker,
            db,
            event_bus,
        }
    }

    /// Parse a compose YAML string into a ComposeFile.
    pub fn parse(yaml: &str) -> Result<ComposeFile, DeployError> {
        serde_yaml::from_str::<ComposeFile>(yaml)
            .map_err(|e| DeployError::ComposeParseError(e.to_string()))
    }

    /// Extract all `${VAR}` and `${VAR:-default}` references from the raw YAML.
    pub fn extract_variables(yaml: &str) -> Vec<(String, Option<String>)> {
        interpolation::extract_variables(yaml)
    }

    /// Interpolate `${VAR}` and `${VAR:-default}` in the YAML string with provided values.
    fn interpolate(yaml: &str, env_vars: &HashMap<String, String>) -> String {
        interpolation::interpolate(yaml, env_vars)
    }

    /// Return service names in dependency order (dependencies first).
    fn dependency_order(compose: &ComposeFile) -> Vec<String> {
        interpolation::dependency_order(compose)
    }

    /// Deploy a compose stack for the given app.
    pub async fn deploy(
        &self,
        app: &App,
        deploy_id: &str,
        yaml: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<ComposeDeployResult, DeployError> {
        // Interpolate variables in the YAML
        let interpolated = Self::interpolate(yaml, env_vars);

        // Parse the interpolated YAML
        let compose = Self::parse(&interpolated)?;

        if compose.services.is_empty() {
            return Err(DeployError::ComposeParseError(
                "No services defined in compose file".to_string(),
            ));
        }

        let network_name = format!("icefall-{}-compose", app.name);

        // Create the isolated bridge network
        if let Err(e) = self.docker.create_network(&network_name).await {
            tracing::debug!("Network {network_name} may already exist: {e}");
        }

        self.emit_event(app, deploy_id, "deploying_compose");

        // Create named volumes if declared at top level
        for vol_name in compose.volumes.keys() {
            let full_name = format!("icefall-{}-{}", app.name, vol_name);
            if let Err(e) = self.docker.create_volume(&full_name).await {
                tracing::debug!("Volume {full_name} may already exist: {e}");
            }
        }

        // Determine deploy order
        let order = Self::dependency_order(&compose);

        let mut results = Vec::new();

        for service_name in &order {
            let service = match compose.services.get(service_name) {
                Some(s) => s,
                None => continue,
            };

            let image = match &service.image {
                Some(img) => img.clone(),
                None => {
                    tracing::warn!(
                        "Skipping service '{service_name}': no image specified (build directive not supported in compose MVP)"
                    );
                    continue;
                }
            };

            // Pull the image
            let (img_name, img_tag) = parse_image_ref(&image);
            tracing::info!("Pulling image {img_name}:{img_tag} for service {service_name}");
            self.docker
                .pull_image(&img_name, &img_tag)
                .await
                .map_err(|e| {
                    DeployError::ContainerCreate(format!(
                        "Failed to pull image {image} for service {service_name}: {e}"
                    ))
                })?;

            // Build container config
            let container_name = format!("icefall-{}-{}", app.name, service_name);

            let env = resolve_service_env(service);
            let ports = resolve_service_ports(service);
            let volumes = resolve_service_volumes(service, &app.name);
            let cmd = resolve_service_command(service);
            let restart_policy = resolve_restart_policy(service);

            let mut labels = HashMap::new();
            labels.insert("icefall.app".to_string(), app.id.clone());
            labels.insert("icefall.stack".to_string(), app.name.clone());
            labels.insert("icefall.service".to_string(), service_name.clone());
            labels.insert("icefall.deploy-id".to_string(), deploy_id.to_string());
            labels.insert("icefall.compose".to_string(), "true".to_string());

            // Don't set network here — we connect manually below with a DNS alias
            let container_config = ContainerConfig {
                name: container_name,
                image: image.clone(),
                env,
                cmd,
                ports,
                volumes,
                memory_bytes: None,
                cpu_shares: None,
                restart_policy: Some(restart_policy),
                labels,
                network: None,
            };

            // Remove any existing containers for this service in this app
            let app_label = format!("icefall.app={}", app.id);
            if let Ok(containers) = self.docker.list_containers(Some(&app_label)).await {
                for c in containers {
                    if c.labels.get("icefall.service").map(|s| s.as_str()) == Some(service_name)
                        && c.labels.get("icefall.compose").map(|s| s.as_str()) == Some("true")
                    {
                        let _ = self.docker.stop_container(&c.id, Some(5)).await;
                        let _ = self.docker.remove_container(&c.id, true).await;
                    }
                }
            }

            let container_id = self
                .docker
                .create_container(&container_config)
                .await
                .map_err(|e| {
                    DeployError::ContainerCreate(format!(
                        "Failed to create container for service {service_name}: {e}"
                    ))
                })?;

            // Connect to the compose network with the service name as a DNS alias
            // so other services can reach this one by name
            self.docker
                .connect_container_to_network_with_alias(&network_name, &container_id, service_name)
                .await
                .map_err(|e| {
                    DeployError::ContainerCreate(format!(
                        "Failed to connect service {service_name} to network: {e}"
                    ))
                })?;

            self.docker.start_container(&container_id).await?;

            tracing::info!(
                "Started compose service '{service_name}' (container {}) for app {}",
                &container_id[..12.min(container_id.len())],
                app.name
            );

            results.push(ComposeServiceResult {
                service_name: service_name.clone(),
                container_id,
                image,
            });
        }

        // Update the deploy record with the first container's ID (for compatibility)
        if let Some(first) = results.first() {
            let _ = self
                .db
                .update_deploy_container_id(deploy_id, &first.container_id)
                .await;
        }

        let _ = self
            .db
            .update_deploy_status(deploy_id, "running", None)
            .await;

        self.emit_event(app, deploy_id, "running");

        Ok(ComposeDeployResult {
            network_name,
            services: results,
        })
    }

    /// Stop and remove all containers belonging to a compose stack.
    pub async fn teardown(&self, app: &App) -> Result<(), DeployError> {
        let label = format!("icefall.app={}", app.id);
        let containers = self.docker.list_containers(Some(&label)).await?;

        for container in containers {
            if container.labels.get("icefall.compose").map(|s| s.as_str()) == Some("true") {
                let _ = self.docker.stop_container(&container.id, Some(10)).await;
                let _ = self.docker.remove_container(&container.id, true).await;
            }
        }

        // Remove the compose network
        let network_name = format!("icefall-{}-compose", app.name);
        let _ = self.docker.remove_network(&network_name).await;

        Ok(())
    }

    fn emit_event(&self, app: &App, deploy_id: &str, status: &str) {
        self.event_bus.emit(
            EventType::DeployStatus,
            Some(&app.id),
            Some(deploy_id),
            serde_json::json!({"status": status, "compose": true}),
        );
    }
}

/// List the service names parsed from a compose YAML (public utility for the frontend API).
pub fn list_compose_services(yaml: &str) -> Result<Vec<String>, DeployError> {
    let compose = ComposeDeployer::parse(yaml)?;
    Ok(compose.services.keys().cloned().collect())
}
