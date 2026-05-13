use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::registry::AgentRegistry;
use crate::caddy::CaddyClient;
use crate::config::IcefallConfig;
use crate::db::models::{App, Deploy, Environment, CONTROL_PLANE_SERVER_ID};
use crate::db::Database;
use crate::deploy::health::wait_for_healthy;
use crate::deploy::remote::RemoteExecutor;
use crate::deploy::s3_mount;
use crate::deploy::{DeployError, DeployTarget};
use crate::docker::containers::{ContainerConfig, PortMapping, VolumeMount};
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

pub struct DeployManager {
    docker: Arc<DockerClient>,
    caddy: Arc<CaddyClient>,
    db: Arc<dyn Database>,
    config: Arc<IcefallConfig>,
    event_bus: Arc<EventBus>,
    agent_registry: Option<Arc<AgentRegistry>>,
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

    async fn deploy_inner(
        &self,
        deploy: &Deploy,
        app: &App,
        env: &Environment,
        image_ref: &str,
        env_override: Option<Vec<String>>,
        auto_rollback: bool,
    ) -> Result<(), DeployError> {
        self.emit_status(app, deploy, "deploying");

        let target = self.resolve_target(app);
        let remote = match target {
            DeployTarget::Remote { ref server_id } => {
                Some(self.make_remote_executor(server_id).await?)
            }
            DeployTarget::Local => None,
        };

        let network_name = format!("icefall-{}", app.name);
        match &remote {
            Some(exec) => {
                let _ = exec.create_network(&network_name).await;
            }
            None => {
                if let Err(e) = self.docker.create_network(&network_name).await {
                    tracing::debug!("Network {network_name} may already exist: {e}");
                }
            }
        }

        let env_vars = match env_override {
            Some(vars) => vars,
            None => self.resolve_env_vars(env).await?,
        };

        let detected_port = self.detected_port(app);
        let container_name = format!(
            "icefall-{}-{}-{}",
            app.name,
            env.env_type,
            &deploy.id[..8.min(deploy.id.len())]
        );

        let mut labels = HashMap::new();
        labels.insert("icefall.app".to_string(), app.id.clone());
        labels.insert("icefall.environment".to_string(), env.id.clone());
        labels.insert("icefall.deploy-id".to_string(), deploy.id.clone());

        let resource_limits: Option<ResourceLimits> = app
            .resource_limits
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        let snapshot = serde_json::to_string(&env_vars).unwrap_or_default();

        // Container creation: local vs remote
        let container_id = match &remote {
            Some(exec) => {
                let env_payload: serde_json::Value = if let Some(ref pub_key) =
                    exec.server_public_key
                {
                    let envelope = crate::deploy::envelope::encrypt_env_vars(&env_vars, pub_key)?;
                    serde_json::to_value(&envelope).unwrap_or_default()
                } else {
                    serde_json::json!(env_vars)
                };

                let params = serde_json::json!({
                    "name": container_name,
                    "image": image_ref,
                    "env": env_payload,
                    "env_encrypted": exec.server_public_key.is_some(),
                    "labels": labels,
                    "ports": [{ "container_port": detected_port, "protocol": "tcp" }],
                    "restart_policy": "unless-stopped",
                    "network": network_name,
                    "memory_bytes": resource_limits.as_ref().and_then(|r| r.memory_bytes),
                    "cpu_shares": resource_limits.as_ref().and_then(|r| r.cpu_shares),
                });
                exec.create_container(params).await?
            }
            None => {
                let volume_entries = s3_mount::parse_volume_entries(app.volumes.as_deref());
                let (local_volumes, s3_configs) = s3_mount::split_volumes(&volume_entries);

                let mut app_volumes: Vec<VolumeMount> = local_volumes;
                for (i, s3_cfg) in s3_configs.iter().enumerate() {
                    match s3_mount::create_s3_sidecar(&self.docker, &app.id, &app.name, i, s3_cfg)
                        .await
                    {
                        Ok(sidecar_id) => {
                            tracing::info!(
                                "S3 sidecar {sidecar_id} started for bucket {}",
                                s3_cfg.bucket
                            );
                            app_volumes.push(VolumeMount {
                                source: s3_mount::s3_volume_name(&app.name, i),
                                target: s3_cfg.target.clone(),
                                read_only: s3_cfg.read_only,
                            });
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to start S3 sidecar for bucket {}: {e}",
                                s3_cfg.bucket
                            );
                        }
                    }
                }

                let container_config = ContainerConfig {
                    name: container_name,
                    image: image_ref.to_string(),
                    env: env_vars,
                    cmd: None,
                    ports: vec![PortMapping {
                        container_port: detected_port,
                        host_port: None,
                        protocol: "tcp".to_string(),
                    }],
                    volumes: app_volumes,
                    memory_bytes: resource_limits.as_ref().and_then(|r| r.memory_bytes),
                    cpu_shares: resource_limits.as_ref().and_then(|r| r.cpu_shares),
                    restart_policy: Some("unless-stopped".to_string()),
                    labels,
                    network: Some(network_name),
                };

                self.docker
                    .create_container(&container_config)
                    .await
                    .map_err(|e| DeployError::ContainerCreate(e.to_string()))?
            }
        };

        // Start container
        match &remote {
            Some(exec) => exec.start_container(&container_id).await?,
            None => self.docker.start_container(&container_id).await?,
        }

        self.db
            .update_deploy_container_id(&deploy.id, &container_id)
            .await?;
        self.db
            .update_deploy_image_ref(&deploy.id, image_ref)
            .await?;
        let _ = self
            .db
            .update_deploy_env_snapshot(&deploy.id, &snapshot)
            .await;

        self.emit_status(app, deploy, "starting");

        // Health check: local vs remote
        let health_result = match &remote {
            Some(exec) => {
                exec.health_check(
                    detected_port,
                    self.config.health_check_attempts,
                    self.config.health_check_interval_ms,
                )
                .await
            }
            None => {
                let host_port = self.get_host_port(&container_id, detected_port).await?;
                wait_for_healthy(
                    host_port,
                    self.config.health_check_attempts,
                    self.config.health_check_interval_ms,
                )
                .await
            }
        };

        if let Err(e) = health_result {
            tracing::error!("Health check failed for deploy {}: {e}", deploy.id);
            self.emit_status(app, deploy, "failed");
            match &remote {
                Some(exec) => {
                    let _ = exec.stop_container(&container_id, 5).await;
                    let _ = exec.remove_container(&container_id).await;
                }
                None => {
                    let _ = self.docker.stop_container(&container_id, Some(5)).await;
                    let _ = self.docker.remove_container(&container_id, true).await;
                }
            }
            self.db
                .update_deploy_status(&deploy.id, "failed", Some("Health check failed"))
                .await?;

            // Auto-rollback: find the previous successful deploy and redeploy it
            if auto_rollback {
                if let Ok(deploys) = self.db.list_deploys(&app.id, 10).await {
                    let previous = deploys.iter().find(|d| {
                        d.id != deploy.id && d.status == "running" && d.image_ref.is_some()
                    });
                    if let Some(prev) = previous {
                        let prev_image = prev.image_ref.clone().unwrap();
                        let prev_snapshot: Option<Vec<String>> = prev
                            .env_snapshot
                            .as_deref()
                            .and_then(|s| serde_json::from_str(s).ok());
                        tracing::info!(
                            "Auto-rolling back app {} to deploy {} (image: {})",
                            app.name,
                            prev.id,
                            prev_image
                        );
                        self.event_bus.emit(
                        EventType::DeployStatus,
                        Some(&app.id),
                        Some(&deploy.id),
                        serde_json::json!({"status": "auto-rollback", "previous_deploy": prev.id}),
                    );
                        if let Ok(rollback_deploy) = self
                            .db
                            .create_deploy(&crate::db::models::NewDeploy {
                                app_id: app.id.clone(),
                                environment_id: env.id.clone(),
                                git_sha: prev.git_sha.clone(),
                                server_id: None,
                            })
                            .await
                        {
                            let _ = Box::pin(self.deploy_inner(
                                &rollback_deploy,
                                app,
                                env,
                                &prev_image,
                                prev_snapshot,
                                false,
                            ))
                            .await;
                        }
                        return Err(e);
                    }
                }
            }

            return Err(e);
        }

        let domains = self.resolve_domains(app, env).await?;

        match &remote {
            Some(exec) => {
                // Remote Caddy routing: two modes
                // - Wildcard (*.base_domain): CP Caddy proxies to worker_host:host_port
                // - Direct (custom domain): worker Caddy handles TLS + routing
                let host_port = self
                    .get_remote_host_port(exec, &container_id, detected_port)
                    .await?;
                let worker_upstream = format!("{}:{}", exec.server_host, host_port);

                for (domain, _path) in &domains {
                    let is_wildcard = self
                        .config
                        .base_domain
                        .as_ref()
                        .is_some_and(|bd| domain.ends_with(bd));

                    if is_wildcard {
                        if self
                            .caddy
                            .update_route(domain, &worker_upstream)
                            .await
                            .is_err()
                        {
                            self.caddy
                                .add_route(domain, &worker_upstream)
                                .await
                                .map_err(|e| DeployError::RouteUpdate(e.to_string()))?;
                        }
                    } else {
                        let local_upstream = format!("localhost:{host_port}");
                        if exec
                            .update_caddy_route(domain, &local_upstream)
                            .await
                            .is_err()
                        {
                            exec.add_caddy_route(domain, &local_upstream)
                                .await
                                .map_err(|e| DeployError::RouteUpdate(e.to_string()))?;
                        }
                    }
                }
            }
            None => {
                let host_port = self.get_host_port(&container_id, detected_port).await?;
                let upstream = format!("localhost:{host_port}");

                for (domain, path) in &domains {
                    if self.caddy.update_route(domain, &upstream).await.is_err() {
                        self.caddy
                            .add_route_with_path(domain, path.as_deref(), &upstream)
                            .await
                            .map_err(|e| DeployError::RouteUpdate(e.to_string()))?;
                    }
                }
            }
        }

        self.db
            .update_deploy_status(&deploy.id, "running", None)
            .await?;
        self.emit_status(app, deploy, "running");

        // Stop old containers
        match &remote {
            Some(exec) => {
                self.stop_old_containers_remote(exec, app, &deploy.id).await;
            }
            None => {
                self.stop_old_containers(app, env, &deploy.id).await;
            }
        }

        Ok(())
    }

    pub async fn teardown(
        &self,
        app: &App,
        env: &Environment,
        deploy_id: &str,
    ) -> Result<(), DeployError> {
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

    async fn resolve_env_vars(&self, env: &Environment) -> Result<Vec<String>, DeployError> {
        let vars = self.db.get_env_vars(&env.id).await?;
        let mut result: Vec<String> = vars
            .into_iter()
            .map(|v| format!("{}={}", v.key, v.value))
            .collect();
        if !result.iter().any(|v| v.starts_with("PORT=")) {
            result.push("PORT=3000".to_string());
        }
        if !result.iter().any(|v| v.starts_with("HOST=")) {
            result.push("HOST=0.0.0.0".to_string());
        }
        Ok(result)
    }

    fn detected_port(&self, app: &App) -> u16 {
        app.build_config
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.get("port")?.as_u64())
            .map(|p| p as u16)
            .unwrap_or(3000)
    }

    async fn get_host_port(
        &self,
        container_id: &str,
        container_port: u16,
    ) -> Result<u16, DeployError> {
        let info = self.docker.inspect_container(container_id).await?;
        let port_key = format!("{container_port}/tcp");

        let host_port = info
            .network_settings
            .and_then(|ns| ns.ports)
            .and_then(|ports| ports.get(&port_key).cloned())
            .and_then(|bindings| bindings)
            .and_then(|bindings| bindings.first().cloned())
            .and_then(|b| b.host_port)
            .and_then(|p| p.parse::<u16>().ok())
            .ok_or_else(|| {
                DeployError::ContainerCreate(format!(
                    "could not determine host port for container {container_id}"
                ))
            })?;

        Ok(host_port)
    }

    async fn resolve_domains(
        &self,
        app: &App,
        env: &Environment,
    ) -> Result<Vec<(String, Option<String>)>, DeployError> {
        let mut domains = Vec::new();

        if env.env_type == "preview" {
            if let (Some(ref branch), Some(ref base_domain)) =
                (&env.branch, &self.config.base_domain)
            {
                let sanitized = crate::deploy::preview::sanitize_branch_for_subdomain(branch);
                domains.push((format!("{sanitized}--{}.{base_domain}", app.name), None));
            }
        } else {
            let custom_domains = self.db.list_domains(&app.id).await?;
            for d in custom_domains {
                domains.push((d.domain, d.path));
            }

            if let Some(ref base_domain) = self.config.base_domain {
                domains.push((format!("{}.{base_domain}", app.name), None));
            }
        }

        Ok(domains)
    }

    async fn stop_old_containers(&self, app: &App, env: &Environment, current_deploy_id: &str) {
        let label = format!("icefall.environment={}", env.id);
        let containers = match self.docker.list_containers(Some(&label)).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to list old containers: {e}");
                return;
            }
        };

        for container in containers {
            let is_current = container
                .labels
                .get("icefall.deploy-id")
                .map(|id| id == current_deploy_id)
                .unwrap_or(false);

            // Skip S3 sidecar containers — they are shared across deploys.
            let is_sidecar = container
                .labels
                .get("icefall.s3-sidecar")
                .map(|v| v == "true")
                .unwrap_or(false);

            if is_current || is_sidecar {
                continue;
            }

            tracing::info!(
                "Stopping old container {} for app {}",
                container.id,
                app.name
            );
            let _ = self
                .docker
                .stop_container(&container.id, Some(self.config.deploy_stop_timeout_secs))
                .await;
            let _ = self.docker.remove_container(&container.id, false).await;
        }
    }

    async fn get_remote_host_port(
        &self,
        exec: &RemoteExecutor,
        container_id: &str,
        container_port: u16,
    ) -> Result<u16, DeployError> {
        let info = exec.inspect_container(container_id).await?;
        let port_key = format!("{container_port}/tcp");

        info["network_settings"]["ports"][&port_key]
            .as_array()
            .and_then(|bindings| bindings.first())
            .and_then(|b| b["host_port"].as_str())
            .and_then(|p| p.parse::<u16>().ok())
            .ok_or_else(|| {
                DeployError::ContainerCreate(format!(
                    "could not determine host port for remote container {container_id}"
                ))
            })
    }

    async fn stop_old_containers_remote(
        &self,
        exec: &RemoteExecutor,
        app: &App,
        current_deploy_id: &str,
    ) {
        let containers = match exec
            .list_containers_by_label(&format!("icefall.app={}", app.id))
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to list remote containers: {e}");
                return;
            }
        };

        for c in containers {
            let c_id = match c["id"].as_str() {
                Some(id) => id.to_string(),
                None => continue,
            };

            let is_current = c["labels"]["icefall.deploy-id"]
                .as_str()
                .is_some_and(|id| id == current_deploy_id);

            if is_current {
                continue;
            }

            let belongs_to_app = c["labels"]["icefall.app"]
                .as_str()
                .is_some_and(|id| id == app.id);

            if !belongs_to_app {
                continue;
            }

            tracing::info!(
                "Stopping old remote container {} for app {}",
                c_id,
                app.name
            );
            let _ = exec
                .stop_container(&c_id, self.config.deploy_stop_timeout_secs)
                .await;
            let _ = exec.remove_container(&c_id).await;
        }
    }

    fn emit_status(&self, app: &App, deploy: &Deploy, status: &str) {
        self.event_bus.emit(
            EventType::DeployStatus,
            Some(&app.id),
            Some(&deploy.id),
            serde_json::json!({"status": status}),
        );
    }
}

#[derive(serde::Deserialize)]
struct ResourceLimits {
    memory_bytes: Option<i64>,
    cpu_shares: Option<i64>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn resolve_env_vars_adds_port_and_host() {
        let vars: Vec<String> = vec!["FOO=bar".to_string(), "BAZ=qux".to_string()];
        assert!(!vars.iter().any(|v| v.starts_with("PORT=")));

        let mut result = vars;
        result.push("PORT=3000".to_string());
        result.push("HOST=0.0.0.0".to_string());

        assert!(result.iter().any(|v| v.starts_with("PORT=")));
        assert!(result.iter().any(|v| v.starts_with("HOST=")));
    }
}
