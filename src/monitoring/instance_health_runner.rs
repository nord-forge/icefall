//! Per-instance health monitoring for multi-instance (load-balanced) apps.
//! Checks each instance's container, transitions status, and replaces failed
//! instances to keep the app at its desired count.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::agent::registry::AgentRegistry;
use crate::api::BuildLockMap;
use crate::caddy::CaddyClient;
use crate::config::IcefallConfig;
use crate::db::models::{App, AppInstance, UpdateAppInstance, CONTROL_PLANE_SERVER_ID};
use crate::db::Database;
use crate::deploy::manager::DeployManager;
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

const CHECK_INTERVAL: Duration = Duration::from_secs(15);
/// Consecutive failed checks before an instance is marked failed.
const FAILURE_THRESHOLD: u32 = 3;

#[derive(Default)]
struct InstanceState {
    consecutive_failures: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_instance_health_runner(
    db: Arc<dyn Database>,
    docker: Arc<DockerClient>,
    caddy: Arc<CaddyClient>,
    config: Arc<IcefallConfig>,
    event_bus: Arc<EventBus>,
    agent_registry: Arc<AgentRegistry>,
    build_locks: Arc<BuildLockMap>,
) {
    tokio::spawn(async move {
        let mut states: HashMap<String, InstanceState> = HashMap::new();

        loop {
            tokio::time::sleep(CHECK_INTERVAL).await;

            let Ok(apps) = db.list_apps().await else {
                continue;
            };

            // Instance ids observed this pass; used to prune stale state.
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

            for app in &apps {
                // Only multi-instance apps participate in instance health.
                if app.desired_instances <= 1 {
                    continue;
                }

                let Ok(instances) = db.list_app_instances(&app.id).await else {
                    continue;
                };

                for instance in &instances {
                    seen.insert(instance.id.clone());
                    check_instance(
                        &db,
                        &docker,
                        &event_bus,
                        &agent_registry,
                        app,
                        instance,
                        &mut states,
                    )
                    .await;
                }

                // Replace failed instances to maintain the desired count.
                maintain_instance_count(
                    &db,
                    &docker,
                    &caddy,
                    &config,
                    &event_bus,
                    &agent_registry,
                    &build_locks,
                    app,
                )
                .await;
            }

            // Drop state for instances that no longer exist.
            states.retain(|id, _| seen.contains(id));
        }
    });
}

async fn check_instance(
    db: &Arc<dyn Database>,
    docker: &Arc<DockerClient>,
    event_bus: &Arc<EventBus>,
    agent_registry: &Arc<AgentRegistry>,
    app: &App,
    instance: &AppInstance,
    states: &mut HashMap<String, InstanceState>,
) {
    // Instances already marked failed are handled by the replacement pass.
    if instance.status == "failed" || instance.status == "stopped" {
        return;
    }

    let healthy = instance_is_running(docker, agent_registry, instance).await;
    let state = states.entry(instance.id.clone()).or_default();

    if healthy {
        if instance.status != "running" {
            let _ = db
                .update_app_instance(
                    &instance.id,
                    &UpdateAppInstance {
                        status: Some("running".to_string()),
                        ..Default::default()
                    },
                )
                .await;
            emit(event_bus, app, &instance.id, "instance.healthy");
        }
        state.consecutive_failures = 0;
    } else {
        state.consecutive_failures += 1;
        if instance.status == "running" {
            let _ = db
                .update_app_instance(
                    &instance.id,
                    &UpdateAppInstance {
                        status: Some("unhealthy".to_string()),
                        ..Default::default()
                    },
                )
                .await;
            emit(event_bus, app, &instance.id, "instance.unhealthy");
        }

        if state.consecutive_failures >= FAILURE_THRESHOLD {
            let _ = db
                .update_app_instance(
                    &instance.id,
                    &UpdateAppInstance {
                        status: Some("failed".to_string()),
                        ..Default::default()
                    },
                )
                .await;
            state.consecutive_failures = 0;
            tracing::warn!(
                "instance {} of app {} marked failed after {FAILURE_THRESHOLD} checks",
                instance.id,
                app.name
            );
        }
    }
}

/// True if the instance's container exists and is running.
async fn instance_is_running(
    docker: &Arc<DockerClient>,
    agent_registry: &Arc<AgentRegistry>,
    instance: &AppInstance,
) -> bool {
    let Some(ref container_id) = instance.container_id else {
        return false;
    };

    if instance.server_id == CONTROL_PLANE_SERVER_ID {
        docker
            .inspect_container(container_id)
            .await
            .ok()
            .and_then(|info| info.state)
            .and_then(|s| s.running)
            .unwrap_or(false)
    } else {
        // Remote: ask the agent to inspect the container.
        match agent_registry
            .send_request(
                &instance.server_id,
                "container.inspect".to_string(),
                serde_json::json!({ "id": container_id }),
            )
            .await
        {
            Ok(crate::agent::protocol::AgentMessage::Response {
                result: Some(val), ..
            }) => val["state"]["running"].as_bool().unwrap_or(false),
            _ => false,
        }
    }
}

/// If an app has fewer healthy instances than desired, replace the failed ones
/// by starting fresh instances from the last successful deploy's image.
#[allow(clippy::too_many_arguments)]
async fn maintain_instance_count(
    db: &Arc<dyn Database>,
    docker: &Arc<DockerClient>,
    caddy: &Arc<CaddyClient>,
    config: &Arc<IcefallConfig>,
    event_bus: &Arc<EventBus>,
    agent_registry: &Arc<AgentRegistry>,
    build_locks: &Arc<BuildLockMap>,
    app: &App,
) {
    let Ok(instances) = db.list_app_instances(&app.id).await else {
        return;
    };

    let healthy = instances.iter().filter(|i| i.status == "running").count() as i64;
    let has_failed = instances.iter().any(|i| i.status == "failed");

    // Nothing to do unless we are short on healthy instances and there is at
    // least one failed instance to clear/replace.
    if healthy >= app.desired_instances || !has_failed {
        return;
    }

    // Resolve the production environment for the replacement deploy.
    let Ok(envs) = db.list_environments(&app.id).await else {
        return;
    };
    let Some(env) = envs.into_iter().find(|e| e.env_type == "production") else {
        tracing::warn!(
            "Cannot replace instances for app {}: no production environment",
            app.name
        );
        return;
    };

    // Serialize with user-initiated scale/deploy operations on this app. If a
    // scale is already in progress, skip this pass — the next tick re-checks.
    let lock = build_locks.acquire(&app.id).await;
    let Ok(_guard) = lock.try_lock() else {
        tracing::debug!(
            "Skipping instance replacement for app {}: scale/deploy in progress",
            app.name
        );
        return;
    };

    let manager = DeployManager::new(
        docker.clone(),
        caddy.clone(),
        db.clone(),
        config.clone(),
        event_bus.clone(),
        Some(agent_registry.clone()),
    );

    match manager.replace_failed_instances(app, &env).await {
        Ok(()) => tracing::info!(
            "app {} instances reconciled toward desired count {}",
            app.name,
            app.desired_instances
        ),
        Err(e) => tracing::error!("Failed to replace instances for app {}: {e}", app.name),
    }
}

fn emit(event_bus: &Arc<EventBus>, app: &App, instance_id: &str, event: &str) {
    event_bus.emit(
        EventType::HealthStatus,
        Some(&app.id),
        Some(instance_id),
        serde_json::json!({ "event": event, "instance_id": instance_id }),
    );
}
