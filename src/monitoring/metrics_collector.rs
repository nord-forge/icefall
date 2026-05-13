use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::db::Database;
use crate::docker::containers::ContainerInfo;
use crate::docker::stats::ContainerStats;
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

const MAX_HISTORY: usize = 360;

#[derive(Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: String,
    pub stats: ContainerStats,
}

pub struct MetricsStore {
    history: RwLock<HashMap<String, VecDeque<MetricsSnapshot>>>,
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            history: RwLock::new(HashMap::new()),
        }
    }

    pub async fn record(&self, app_id: &str, stats: ContainerStats) {
        let mut history = self.history.write().await;
        let buf = history
            .entry(app_id.to_string())
            .or_insert_with(|| VecDeque::with_capacity(MAX_HISTORY + 1));

        buf.push_back(MetricsSnapshot {
            timestamp: crate::db::models::now_iso8601(),
            stats,
        });

        if buf.len() > MAX_HISTORY {
            buf.pop_front();
        }
    }

    pub async fn get_current(&self, app_id: &str) -> Option<MetricsSnapshot> {
        let history = self.history.read().await;
        history.get(app_id)?.back().cloned()
    }

    pub async fn get_history(&self, app_id: &str) -> Vec<MetricsSnapshot> {
        let history = self.history.read().await;
        history
            .get(app_id)
            .map(|buf| buf.iter().cloned().collect())
            .unwrap_or_default()
    }
}

pub fn spawn_metrics_collector(
    docker: Arc<DockerClient>,
    db: Arc<dyn Database>,
    event_bus: Arc<EventBus>,
    metrics_store: Arc<MetricsStore>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let Ok(apps) = db.list_apps().await else {
                continue;
            };

            for app in &apps {
                let label = format!("icefall.app={}", app.id);
                let Ok(containers) = docker.list_containers(Some(&label)).await else {
                    continue;
                };

                let running: Vec<&ContainerInfo> =
                    containers.iter().filter(|c| c.state == "running").collect();

                for container in running {
                    let Ok(stats) = docker.get_stats(&container.id).await else {
                        continue;
                    };

                    metrics_store.record(&app.id, stats.clone()).await;

                    event_bus.emit(
                        EventType::HealthStatus,
                        Some(&app.id),
                        None,
                        serde_json::json!({
                            "type": "container.metrics",
                            "cpu_percent": stats.cpu_percent,
                            "memory_usage_bytes": stats.memory_usage_bytes,
                            "memory_limit_bytes": stats.memory_limit_bytes,
                            "network_rx_bytes": stats.network_rx_bytes,
                            "network_tx_bytes": stats.network_tx_bytes,
                        }),
                    );
                }
            }
        }
    });
}
