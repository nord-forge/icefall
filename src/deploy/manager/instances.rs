use std::collections::HashMap;
use std::time::Duration;

use crate::db::models::{
    App, AppInstance, Deploy, Environment, NewAppInstance, UpdateAppInstance,
    CONTROL_PLANE_SERVER_ID,
};
use crate::deploy::{DeployError, DeployTarget};
use crate::docker::containers::{ContainerConfig, PortMapping};
use crate::events::EventType;

use super::{DeployManager, ResourceLimits};

impl DeployManager {
    /// Deploy an app across multiple servers to satisfy `app.desired_instances`,
    /// reconciling current to desired state. New instances are started one at a
    /// time (rolling) and old ones torn down only once the new ones are healthy.
    pub async fn deploy_instances(
        &self,
        deploy: &Deploy,
        app: &App,
        env: &Environment,
        image_ref: &str,
    ) -> Result<(), DeployError> {
        let desired = app.desired_instances.max(1) as usize;
        self.emit_status(app, deploy, "deploying");

        // Snapshot existing instances up front so we can tear them down only
        // after the replacements are healthy.
        let existing = self.db.list_app_instances(&app.id).await?;

        let targets = self.select_instance_targets(app, desired).await?;
        if targets.is_empty() {
            self.fail_deploy(app, deploy, "no servers available for deployment")
                .await?;
            return Err(DeployError::RemoteOp(
                "no servers available for deployment".to_string(),
            ));
        }

        // Export the built image once if any target is remote.
        let has_remote = targets.iter().any(|t| !matches!(t, DeployTarget::Local));
        let image_tar = if has_remote {
            Some(self.docker.export_image(image_ref).await?)
        } else {
            None
        };

        let detected_port = self.detected_port(app);
        let env_vars = self.resolve_env_vars(env).await?;

        let mut started_count: usize = 0;

        // Rolling deploy: bring up new instances one at a time.
        for (idx, target) in targets.iter().enumerate() {
            match self
                .start_instance(
                    deploy,
                    app,
                    env,
                    image_ref,
                    image_tar.as_deref(),
                    target,
                    detected_port,
                    &env_vars,
                    idx,
                )
                .await
            {
                Ok(()) => started_count += 1,
                Err(e) => {
                    tracing::error!(
                        "Failed to start instance {} of {} for app {}: {e}",
                        idx + 1,
                        targets.len(),
                        app.name
                    );
                    // No new instance came up at all — abort, leaving the old
                    // deployment untouched so the app keeps serving.
                    if started_count == 0 {
                        self.fail_deploy(app, deploy, &format!("instance start failed: {e}"))
                            .await?;
                        return Err(e);
                    }
                    // Otherwise continue in degraded mode (fewer than desired).
                }
            }
        }

        // New instances are healthy — now tear down the previous generation.
        for old in &existing {
            if let Err(e) = self.destroy_instance_container(old).await {
                tracing::warn!(
                    "Failed to tear down old instance {} for app {}: {e}",
                    old.id,
                    app.name
                );
            }
            let _ = self.db.delete_app_instance(&old.id).await;
        }

        // Rebuild Caddy from the final running set.
        self.rebuild_caddy_routes(app, env).await?;

        self.db
            .update_deploy_status(&deploy.id, "running", None)
            .await?;
        self.emit_status(app, deploy, "running");

        tracing::info!(
            "app {} deployed across {} instance(s) (desired {})",
            app.name,
            started_count,
            desired
        );
        Ok(())
    }

    /// Rebuild the Caddy upstream list for an app from its currently running
    /// instances. Removes the route entirely when no instances are running.
    pub async fn rebuild_caddy_routes(
        &self,
        app: &App,
        env: &Environment,
    ) -> Result<(), DeployError> {
        let instances = self.db.list_app_instances(&app.id).await?;

        // Resolve server hosts once for any remote instances.
        let servers = self
            .db
            .list_servers()
            .await
            .map_err(|e| DeployError::RemoteOp(e.to_string()))?;
        let host_by_id: HashMap<String, String> =
            servers.into_iter().map(|s| (s.id, s.host)).collect();

        let upstreams: Vec<String> = instances
            .iter()
            .filter(|i| i.status == "running")
            .filter_map(|i| instance_upstream(i, &host_by_id))
            .collect();

        let domains = self.resolve_domains(app, env).await?;

        if upstreams.is_empty() {
            for (domain, _path) in &domains {
                let _ = self.caddy.remove_route(domain).await;
            }
            return Ok(());
        }

        let health_path = if app.lb_health_check_path.is_empty() {
            "/".to_string()
        } else {
            app.lb_health_check_path.clone()
        };
        for (domain, _path) in &domains {
            self.caddy
                .set_route_balanced(domain, &upstreams, &app.lb_policy, &health_path)
                .await
                .map_err(|e| DeployError::RouteUpdate(e.to_string()))?;
        }
        Ok(())
    }

    async fn fail_deploy(
        &self,
        app: &App,
        deploy: &Deploy,
        message: &str,
    ) -> Result<(), DeployError> {
        self.emit_status(app, deploy, "failed");
        self.db
            .update_deploy_status(&deploy.id, "failed", Some(message))
            .await?;
        Ok(())
    }

    /// Choose servers for `desired` instances: the app's primary server first,
    /// then round-robin across online servers by free capacity. `desired` may
    /// exceed the server count — a server can host multiple instances.
    async fn select_instance_targets(
        &self,
        app: &App,
        desired: usize,
    ) -> Result<Vec<DeployTarget>, DeployError> {
        let primary = app
            .server_id
            .clone()
            .unwrap_or_else(|| CONTROL_PLANE_SERVER_ID.to_string());

        let mut servers = self
            .db
            .list_servers()
            .await
            .map_err(|e| DeployError::RemoteOp(e.to_string()))?;
        servers.retain(|s| s.status == "online");

        if servers.is_empty() {
            return Ok(Vec::new());
        }

        // Order: primary first, then by descending free capacity.
        servers.sort_by(|a, b| {
            if a.id == primary {
                std::cmp::Ordering::Less
            } else if b.id == primary {
                std::cmp::Ordering::Greater
            } else {
                server_free_capacity(b)
                    .partial_cmp(&server_free_capacity(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // Round-robin assignment so desired > server count is supported.
        let targets = (0..desired)
            .map(|i| {
                let server = &servers[i % servers.len()];
                if server.id == CONTROL_PLANE_SERVER_ID {
                    DeployTarget::Local
                } else {
                    DeployTarget::Remote {
                        server_id: server.id.clone(),
                    }
                }
            })
            .collect();

        Ok(targets)
    }

    /// Start a single app instance on one target server, recording an
    /// `app_instances` row and returning its Caddy upstream.
    #[allow(clippy::too_many_arguments)]
    async fn start_instance(
        &self,
        deploy: &Deploy,
        app: &App,
        env: &Environment,
        image_ref: &str,
        image_tar: Option<&[u8]>,
        target: &DeployTarget,
        detected_port: u16,
        env_vars: &[String],
        index: usize,
    ) -> Result<(), DeployError> {
        let server_id = match target {
            DeployTarget::Local => CONTROL_PLANE_SERVER_ID.to_string(),
            DeployTarget::Remote { server_id } => server_id.clone(),
        };

        // Create the instance row up front so failures are observable.
        let instance = self
            .db
            .create_app_instance(&NewAppInstance {
                app_id: app.id.clone(),
                server_id: server_id.clone(),
                status: "deploying".to_string(),
                container_id: None,
                host_port: None,
            })
            .await?;

        match self
            .start_instance_inner(
                deploy,
                app,
                env,
                image_ref,
                image_tar,
                target,
                detected_port,
                env_vars,
                index,
            )
            .await
        {
            Ok((container_id, host_port)) => {
                self.db
                    .update_app_instance(
                        &instance.id,
                        &UpdateAppInstance {
                            status: Some("running".to_string()),
                            container_id: Some(Some(container_id)),
                            host_port: Some(Some(i64::from(host_port))),
                        },
                    )
                    .await?;
                self.emit_instance_event(app, &instance.id, "instance.healthy");
                Ok(())
            }
            Err(e) => {
                // Best-effort cleanup of any partially-created container, then
                // delete the row so it does not leak as a stale `failed`
                // record. The deploy reports degraded/failed via the caller.
                let refreshed = self.db.get_app_instance(&instance.id).await.ok().flatten();
                if let Some(inst) = refreshed {
                    let _ = self.destroy_instance_container(&inst).await;
                }
                let _ = self.db.delete_app_instance(&instance.id).await;
                Err(e)
            }
        }
    }

    /// Create + start the container for one instance and run its health check.
    /// Returns `(container_id, host_port)`.
    #[allow(clippy::too_many_arguments)]
    async fn start_instance_inner(
        &self,
        deploy: &Deploy,
        app: &App,
        env: &Environment,
        image_ref: &str,
        image_tar: Option<&[u8]>,
        target: &DeployTarget,
        detected_port: u16,
        env_vars: &[String],
        index: usize,
    ) -> Result<(String, u16), DeployError> {
        let remote = match target {
            DeployTarget::Remote { server_id } => Some(self.make_remote_executor(server_id).await?),
            DeployTarget::Local => None,
        };

        let network_name = format!("icefall-{}", app.name);
        match &remote {
            Some(exec) => {
                let _ = exec.create_network(&network_name).await;
            }
            None => {
                let _ = self.docker.create_network(&network_name).await;
            }
        }

        // Container name includes the deploy id and instance index, keeping it
        // unique across re-deploys (old generation is removed explicitly).
        let container_name = format!(
            "icefall-{}-{}-{}-i{}",
            app.name,
            env.env_type,
            &deploy.id[..8.min(deploy.id.len())],
            index
        );

        let mut labels = HashMap::new();
        labels.insert("icefall.app".to_string(), app.id.clone());
        labels.insert("icefall.environment".to_string(), env.id.clone());
        labels.insert("icefall.deploy-id".to_string(), deploy.id.clone());
        labels.insert("icefall.instance-index".to_string(), index.to_string());

        let resource_limits: Option<ResourceLimits> = app
            .resource_limits
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        let (container_id, host_port) = match &remote {
            Some(exec) => {
                // Transfer the image to the remote server before starting.
                if let Some(tar) = image_tar {
                    exec.load_image(
                        tar,
                        self.config.image_transfer_chunk_bytes,
                        Duration::from_secs(self.config.build_timeout_secs),
                    )
                    .await?;
                }

                let env_payload: serde_json::Value = if let Some(ref pub_key) =
                    exec.server_public_key
                {
                    let envelope = crate::deploy::envelope::encrypt_env_vars(env_vars, pub_key)?;
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
                let container_id = exec.create_container(params).await?;
                exec.start_container(&container_id).await?;

                exec.health_check(
                    detected_port,
                    self.config.health_check_attempts,
                    self.config.health_check_interval_ms,
                )
                .await?;

                let host_port = self
                    .get_remote_host_port(exec, &container_id, detected_port)
                    .await?;
                (container_id, host_port)
            }
            None => {
                let internal_hostname = format!("{}-i{}.icefall.internal", app.name, index);
                let container_config = ContainerConfig {
                    name: container_name,
                    image: image_ref.to_string(),
                    env: env_vars.to_vec(),
                    cmd: None,
                    ports: vec![PortMapping {
                        container_port: detected_port,
                        host_port: None,
                        protocol: "tcp".to_string(),
                    }],
                    volumes: Vec::new(),
                    memory_bytes: resource_limits.as_ref().and_then(|r| r.memory_bytes),
                    cpu_shares: resource_limits.as_ref().and_then(|r| r.cpu_shares),
                    restart_policy: Some("unless-stopped".to_string()),
                    labels,
                    network: Some(network_name),
                    hostname: Some(internal_hostname),
                };

                let container_id = self
                    .docker
                    .create_container(&container_config)
                    .await
                    .map_err(|e| DeployError::ContainerCreate(e.to_string()))?;
                self.docker.start_container(&container_id).await?;

                let host_port = self.get_host_port(&container_id, detected_port).await?;
                crate::deploy::health::wait_for_healthy(
                    host_port,
                    self.config.health_check_attempts,
                    self.config.health_check_interval_ms,
                )
                .await?;

                (container_id, host_port)
            }
        };

        Ok((container_id, host_port))
    }

    fn emit_instance_event(&self, app: &App, instance_id: &str, event: &str) {
        // Instance lifecycle events ride the HealthStatus channel, matching the
        // instance health runner so a single SSE subscription sees all of them.
        self.event_bus.emit(
            EventType::HealthStatus,
            Some(&app.id),
            Some(instance_id),
            serde_json::json!({ "event": event, "instance_id": instance_id }),
        );
    }

    /// Stop and remove an instance's container (local or remote). Does NOT
    /// delete the DB row — callers decide that.
    async fn destroy_instance_container(&self, instance: &AppInstance) -> Result<(), DeployError> {
        let Some(ref container_id) = instance.container_id else {
            return Ok(());
        };
        if instance.server_id == CONTROL_PLANE_SERVER_ID {
            let _ = self
                .docker
                .stop_container(container_id, Some(self.config.deploy_stop_timeout_secs))
                .await;
            let _ = self.docker.remove_container(container_id, true).await;
        } else if let Ok(exec) = self.make_remote_executor(&instance.server_id).await {
            let _ = exec
                .stop_container(container_id, self.config.deploy_stop_timeout_secs)
                .await;
            let _ = exec.remove_container(container_id).await;
        }
        Ok(())
    }

    /// Reconcile a multi-instance app back up to `desired_instances` after
    /// failures: remove failed instances, then fill the shortfall with the image
    /// from the latest successful deploy (no rebuild) and rebuild Caddy.
    pub async fn replace_failed_instances(
        &self,
        app: &App,
        env: &Environment,
    ) -> Result<(), DeployError> {
        let instances = self.db.list_app_instances(&app.id).await?;

        // Clear failed instances (container + row).
        for instance in instances.iter().filter(|i| i.status == "failed") {
            let _ = self.destroy_instance_container(instance).await;
            let _ = self.db.delete_app_instance(&instance.id).await;
            self.emit_instance_event(app, &instance.id, "instance.replaced");
        }

        let running = instances.iter().filter(|i| i.status == "running").count() as i64;
        let shortfall = (app.desired_instances - running).max(0) as usize;
        if shortfall == 0 {
            return Ok(());
        }

        // Reuse the image from the latest successful deploy.
        let deploys = self.db.list_deploys(&app.id, 20).await?;
        let Some(source) = deploys
            .iter()
            .find(|d| d.status == "running" && d.image_ref.is_some())
        else {
            tracing::warn!(
                "Cannot replace instances for app {}: no prior successful deploy with an image",
                app.name
            );
            return Err(DeployError::RemoteOp(
                "no image available for instance replacement".to_string(),
            ));
        };
        let image_ref = source.image_ref.clone().expect("filtered by is_some()");
        let deploy = source.clone();

        let targets = self.select_instance_targets(app, shortfall).await?;
        if targets.is_empty() {
            return Err(DeployError::RemoteOp(
                "no servers available for instance replacement".to_string(),
            ));
        }

        let has_remote = targets.iter().any(|t| !matches!(t, DeployTarget::Local));
        let image_tar = if has_remote {
            Some(self.docker.export_image(&image_ref).await?)
        } else {
            None
        };
        let detected_port = self.detected_port(app);
        let env_vars = self.resolve_env_vars(env).await?;

        // Instance indices continue past the highest in-use index to avoid
        // container name collisions with surviving instances.
        let base_index = instances.len();
        for (offset, target) in targets.iter().enumerate() {
            if let Err(e) = self
                .start_instance(
                    &deploy,
                    app,
                    env,
                    &image_ref,
                    image_tar.as_deref(),
                    target,
                    detected_port,
                    &env_vars,
                    base_index + offset,
                )
                .await
            {
                tracing::error!(
                    "Failed to start replacement instance for app {}: {e}",
                    app.name
                );
            }
        }

        self.rebuild_caddy_routes(app, env).await?;
        Ok(())
    }

    /// Tear down a single instance: stop/remove its container, delete the row,
    /// and rebuild the app's Caddy routes so the dead upstream is dropped.
    pub async fn teardown_instance(&self, instance: &AppInstance) -> Result<(), DeployError> {
        self.destroy_instance_container(instance).await?;
        self.db.delete_app_instance(&instance.id).await?;

        // Rebuild Caddy from the remaining running instances. Best-effort:
        // failure to refresh routes should not block instance removal.
        if let Ok(Some(app)) = self.db.get_app(&instance.app_id).await {
            if let Ok(envs) = self.db.list_environments(&app.id).await {
                if let Some(env) = envs.into_iter().find(|e| e.env_type == "production") {
                    if let Err(e) = self.rebuild_caddy_routes(&app, &env).await {
                        tracing::warn!(
                            "Failed to rebuild Caddy routes after removing instance {}: {e}",
                            instance.id
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

/// Caddy upstream (`host:port`) for a running instance. Local instances use
/// `localhost`; remote instances use the server's host. Returns `None` if the
/// instance has no host port or its server host cannot be resolved.
fn instance_upstream(
    instance: &AppInstance,
    host_by_id: &HashMap<String, String>,
) -> Option<String> {
    let port = instance.host_port?;
    if instance.server_id == CONTROL_PLANE_SERVER_ID {
        Some(format!("localhost:{port}"))
    } else {
        let host = host_by_id.get(&instance.server_id)?;
        Some(format!("{host}:{port}"))
    }
}

/// Estimate free capacity for a server from its `resources` JSON blob.
/// Returns a fraction in `[0.0, 1.0]`; unknown resources sort last.
fn server_free_capacity(server: &crate::db::models::Server) -> f64 {
    server
        .resources
        .as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| {
            let cpu = v.get("cpu_percent")?.as_f64()?;
            Some(1.0 - (cpu / 100.0).clamp(0.0, 1.0))
        })
        .unwrap_or(0.0)
}
