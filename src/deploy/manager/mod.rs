mod cleanup;
mod deploy_inner;
mod environment;
mod instances;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::agent::registry::AgentRegistry;
use crate::caddy::CaddyClient;
use crate::config::IcefallConfig;
use crate::db::models::{App, Deploy, Environment, CONTROL_PLANE_SERVER_ID};
use crate::db::Database;
use crate::deploy::remote::RemoteExecutor;
use crate::deploy::{DeployError, DeployTarget};
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

pub struct DeployManager {
    pub(super) docker: Arc<DockerClient>,
    pub(super) caddy: Arc<CaddyClient>,
    pub(super) db: Arc<dyn Database>,
    pub(super) config: Arc<IcefallConfig>,
    pub(super) event_bus: Arc<EventBus>,
    pub(super) agent_registry: Option<Arc<AgentRegistry>>,
}

#[derive(serde::Deserialize)]
pub(super) struct ResourceLimits {
    pub(super) memory_bytes: Option<i64>,
    pub(super) cpu_shares: Option<i64>,
}

impl DeployManager {
    pub fn new(
        docker: Arc<DockerClient>,
        caddy: Arc<CaddyClient>,
        db: Arc<dyn Database>,
        config: Arc<IcefallConfig>,
        event_bus: Arc<EventBus>,
        agent_registry: Option<Arc<AgentRegistry>>,
    ) -> Self {
        Self {
            docker,
            caddy,
            db,
            config,
            event_bus,
            agent_registry,
        }
    }

    pub fn resolve_target(&self, app: &App) -> DeployTarget {
        match app.server_id.as_deref() {
            None | Some(CONTROL_PLANE_SERVER_ID) => DeployTarget::Local,
            Some(id) => DeployTarget::Remote {
                server_id: id.to_string(),
            },
        }
    }

    pub async fn make_remote_executor(
        &self,
        server_id: &str,
    ) -> Result<RemoteExecutor, DeployError> {
        let registry = self
            .agent_registry
            .as_ref()
            .ok_or_else(|| DeployError::AgentOffline("agent registry not available".to_string()))?
            .clone();

        let server = self
            .db
            .get_server(server_id)
            .await
            .map_err(|e| DeployError::RemoteOp(e.to_string()))?
            .ok_or_else(|| DeployError::AgentOffline(format!("server {server_id} not found")))?;

        if server.status != "online" {
            return Err(DeployError::AgentOffline(format!(
                "server '{}' is {} (expected online)",
                server.name, server.status
            )));
        }

        Ok(RemoteExecutor::new(
            registry,
            server_id.to_string(),
            server.host,
            server.public_key,
        ))
    }

    pub async fn deploy(
        &self,
        deploy: &Deploy,
        app: &App,
        env: &Environment,
        image_ref: &str,
    ) -> Result<(), DeployError> {
        self.deploy_inner(deploy, app, env, image_ref, None, true)
            .await
    }

    pub async fn deploy_with_env(
        &self,
        deploy: &Deploy,
        app: &App,
        env: &Environment,
        image_ref: &str,
        env_override: Option<Vec<String>>,
    ) -> Result<(), DeployError> {
        self.deploy_inner(deploy, app, env, image_ref, env_override, true)
            .await
    }

    pub async fn teardown(
        &self,
        app: &App,
        env: &Environment,
        deploy_id: &str,
    ) -> Result<(), DeployError> {
        use crate::deploy::s3_mount;

        let label = format!("icefall.environment={}", env.id);
        let containers = self.docker.list_containers(Some(&label)).await?;

        for container in containers {
            let _ = self
                .docker
                .stop_container(&container.id, Some(self.config.deploy_stop_timeout_secs))
                .await;
            let _ = self.docker.remove_container(&container.id, true).await;
        }

        // Stop and remove any S3 sidecar containers for this app.
        s3_mount::stop_s3_sidecars(&self.docker, &app.id).await;

        // Clean up S3 shared volumes.
        let volume_entries = s3_mount::parse_volume_entries(app.volumes.as_deref());
        let (_, s3_configs) = s3_mount::split_volumes(&volume_entries);
        if !s3_configs.is_empty() {
            s3_mount::remove_s3_volumes(&self.docker, &app.name, s3_configs.len()).await;
        }

        let domains = self.resolve_domains(app, env).await?;
        for (domain, _path) in &domains {
            let _ = self.caddy.remove_route(domain).await;
        }

        if !deploy_id.is_empty() {
            let _ = self
                .db
                .update_deploy_status(deploy_id, "stopped", None)
                .await;
        }

        Ok(())
    }

    pub(super) fn emit_status(&self, app: &App, deploy: &Deploy, status: &str) {
        self.event_bus.emit(
            EventType::DeployStatus,
            Some(&app.id),
            Some(&deploy.id),
            serde_json::json!({"status": status}),
        );
    }
}
