use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::db::Database;
use crate::docker::containers::ContainerInfo;
use crate::docker::stats::ContainerStats;
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

/// Default live-history window per container (~1h at 10s cadence).
const DEFAULT_MAX_HISTORY: usize = 360;

#[derive(Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: String,
    pub stats: ContainerStats,
}

pub struct MetricsStore {
    history: RwLock<HashMap<String, VecDeque<MetricsSnapshot>>>,
    /// Per-container ring-buffer length. Lower in low-memory mode.
    max_history: usize,
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsStore {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_HISTORY)
    }

    /// Construct with an explicit per-container history length.
    pub fn with_capacity(max_history: usize) -> Self {
        Self {
            history: RwLock::new(HashMap::new()),
            max_history: max_history.max(1),
        }
    }

    pub async fn record(&self, app_id: &str, stats: ContainerStats) {
        let mut history = self.history.write().await;
        let buf = history
            .entry(app_id.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.max_history + 1));

        buf.push_back(MetricsSnapshot {
            timestamp: crate::db::models::now_iso8601(),
            stats,
        });

        while buf.len() > self.max_history {
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

/// Live metrics are emitted every 10s. Persisting at that rate would write
/// ~8.6k rows/container/day; once per minute (every 6th tick) is plenty of
/// resolution for the 7-day right-sizing analysis (IF-191) at 1/6th the rows.
const PERSIST_EVERY_N_TICKS: u64 = 6;
/// Drop persisted samples older than this. The analysis window (7 days) is
/// shorter, so this keeps a little extra history without unbounded growth.
const METRICS_RETENTION_DAYS: i64 = 14;

pub fn spawn_metrics_collector(
    docker: Arc<DockerClient>,
    db: Arc<dyn Database>,
    event_bus: Arc<EventBus>,
    metrics_store: Arc<MetricsStore>,
) {
    tokio::spawn(async move {
        let mut tick: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tick = tick.wrapping_add(1);
            let persist = tick % PERSIST_EVERY_N_TICKS == 0;

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

                    // Persist a coarse sample for the resource packer (IF-191).
                    if persist {
                        let _ = db
                            .record_container_metrics(
                                &crate::db::models::NewContainerMetricsRecord {
                                    app_id: app.id.clone(),
                                    cpu_percent: stats.cpu_percent,
                                    memory_usage_bytes: stats.memory_usage_bytes as i64,
                                    memory_limit_bytes: stats.memory_limit_bytes as i64,
                                },
                            )
                            .await;
                    }

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

            // Prune occasionally — piggyback on the persist tick so it runs
            // about once a minute, cheap given the indexed delete.
            if persist {
                let _ = db.prune_container_metrics(METRICS_RETENTION_DAYS).await;
            }
        }
    });
}
