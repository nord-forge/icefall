use std::collections::HashMap;

use crate::db::models::{App, Deploy, Environment};
use crate::deploy::health::wait_for_healthy;
use crate::deploy::s3_mount;
use crate::deploy::{DeployError, DeployTarget};
use crate::docker::containers::{ContainerConfig, PortMapping, VolumeMount};
use crate::events::EventType;

use super::{DeployManager, ResourceLimits};

impl DeployManager {
    pub(super) async fn deploy_inner(
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
}
