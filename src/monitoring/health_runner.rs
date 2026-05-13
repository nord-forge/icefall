use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;

use crate::db::models::{HealthCheck, NewHealthCheckEvent};
use crate::db::Database;
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

struct AppHealthState {
    consecutive_failures: u32,
    restart_count: u32,
    last_status: String,
    last_checked: Instant,
}

pub fn spawn_health_runner(
    db: Arc<dyn Database>,
    docker: Arc<DockerClient>,
    event_bus: Arc<EventBus>,
) {
    tokio::spawn(async move {
        let mut states: HashMap<String, AppHealthState> = HashMap::new();

        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let apps = match db.list_apps().await {
                Ok(a) => a,
                Err(_) => continue,
            };

            for app in &apps {
                let checks = match db.get_health_checks(&app.id).await {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if checks.is_empty() {
                    continue;
                }

                let is_native = app.deploy_mode == "native";

                let container_label = format!("icefall.app={}", app.id);
                let containers = if is_native {
                    vec![]
                } else {
                    match docker.list_containers(Some(&container_label)).await {
                        Ok(c) => c,
                        Err(_) => continue,
                    }
                };
                let running_container = containers.iter().find(|c| c.state == "running");

                for check in &checks {
                    let due = is_check_due(check, &states);
                    if !due {
                        continue;
                    }

                    let healthy = if is_native {
                        run_http_check(check).await
                    } else if let Some(container) = running_container {
                        run_check(check, &docker, &container.id).await
                    } else {
                        false
                    };

                    let state = states.entry(check.id.clone()).or_insert(AppHealthState {
                        consecutive_failures: 0,
                        restart_count: 0,
                        last_status: "unknown".to_string(),
                        last_checked: Instant::now(),
                    });
                    state.last_checked = Instant::now();

                    let status = if healthy { "healthy" } else { "unhealthy" };

                    if healthy {
                        if state.last_status == "unhealthy" {
                            event_bus.emit(
                                EventType::HealthStatus,
                                Some(&app.id),
                                None,
                                serde_json::json!({"status": "recovered", "app": &app.name}),
                            );
                        }
                        state.consecutive_failures = 0;
                        state.last_status = "healthy".to_string();
                    } else {
                        state.consecutive_failures += 1;
                        state.last_status = "unhealthy".to_string();

                        if state.consecutive_failures >= check.failure_threshold as u32 {
                            event_bus.emit(
                                EventType::HealthStatus,
                                Some(&app.id),
                                None,
                                serde_json::json!({
                                    "status": "unhealthy",
                                    "app": &app.name,
                                    "failures": state.consecutive_failures,
                                }),
                            );

                            if check.auto_restart && state.restart_count < 5 {
                                if let Some(container) = running_container {
                                    let _ = docker.restart_container(&container.id).await;
                                    state.restart_count += 1;
                                    state.consecutive_failures = 0;
                                    tracing::info!(
                                        "Auto-restarted container for app {} (attempt {})",
                                        app.name,
                                        state.restart_count
                                    );
                                }
                            }
                        }
                    }

                    let _ = db
                        .record_health_event(&NewHealthCheckEvent {
                            health_check_id: check.id.clone(),
                            status: status.to_string(),
                        })
                        .await;
                }
            }
        }
    });
}

async fn run_http_check(check: &HealthCheck) -> bool {
    let config = check
        .config
        .as_deref()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
        .unwrap_or_default();

    let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
    let path = config
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("/");

    let url = format!("http://127.0.0.1:{port}{path}");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.get(&url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn run_check(check: &HealthCheck, docker: &DockerClient, container_id: &str) -> bool {
    match check.check_type.as_str() {
        "tcp" => {
            let port = check
                .config
                .as_deref()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                .and_then(|v| v.get("port")?.as_u64())
                .unwrap_or(3000) as u16;

            let info = match docker.inspect_container(container_id).await {
                Ok(i) => i,
                Err(_) => return false,
            };

            let host_port = info
                .network_settings
                .and_then(|ns| ns.ports)
                .and_then(|ports| {
                    let key = format!("{port}/tcp");
                    ports.get(&key)?.as_ref()?.first()?.host_port.clone()
                })
                .and_then(|p| p.parse::<u16>().ok());

            let host_port = match host_port {
                Some(p) => p,
                None => return false,
            };

            tokio::time::timeout(
                Duration::from_secs(5),
                TcpStream::connect(format!("127.0.0.1:{host_port}")),
            )
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false)
        }
        "docker" => {
            let info = match docker.inspect_container(container_id).await {
                Ok(i) => i,
                Err(_) => return false,
            };

            info.state
                .and_then(|s| s.health)
                .and_then(|h| h.status)
                .map(|s| s.to_string().contains("healthy"))
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn is_check_due(check: &HealthCheck, states: &HashMap<String, AppHealthState>) -> bool {
    match states.get(&check.id) {
        Some(state) => {
            state.last_checked.elapsed() >= Duration::from_secs(check.interval_secs as u64)
        }
        None => true,
    }
}

pub fn calculate_uptime(events: &[crate::db::models::HealthCheckEvent]) -> f64 {
    if events.is_empty() {
        return 100.0;
    }
    let healthy_count = events.iter().filter(|e| e.status == "healthy").count();
    (healthy_count as f64 / events.len() as f64) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::HealthCheckEvent;

    #[test]
    fn uptime_all_healthy() {
        let events: Vec<HealthCheckEvent> = (0..10)
            .map(|i| HealthCheckEvent {
                id: format!("e{i}"),
                health_check_id: "hc1".to_string(),
                status: "healthy".to_string(),
                checked_at: "2026-05-07T00:00:00Z".to_string(),
            })
            .collect();
        assert!((calculate_uptime(&events) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn uptime_mixed() {
        let mut events: Vec<HealthCheckEvent> = (0..8)
            .map(|i| HealthCheckEvent {
                id: format!("e{i}"),
                health_check_id: "hc1".to_string(),
                status: "healthy".to_string(),
                checked_at: "2026-05-07T00:00:00Z".to_string(),
            })
            .collect();
        events.push(HealthCheckEvent {
            id: "e8".to_string(),
            health_check_id: "hc1".to_string(),
            status: "unhealthy".to_string(),
            checked_at: "2026-05-07T00:00:00Z".to_string(),
        });
        events.push(HealthCheckEvent {
            id: "e9".to_string(),
            health_check_id: "hc1".to_string(),
            status: "unhealthy".to_string(),
            checked_at: "2026-05-07T00:00:00Z".to_string(),
        });
        assert!((calculate_uptime(&events) - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn uptime_empty() {
        assert!((calculate_uptime(&[]) - 100.0).abs() < f64::EPSILON);
    }
}
