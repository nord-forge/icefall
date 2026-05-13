use std::collections::VecDeque;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::warn;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::Database;

const SERVER_HISTORY_CAPACITY: usize = 120;
const COLLECT_INTERVAL_SECS: u64 = 2;
const SQLITE_WRITE_TICKS: u64 = 15; // every 30s (15 * 2s)
const PRUNE_TICKS: u64 = 1800; // every hour (1800 * 2s)

#[derive(Clone, serde::Serialize, Default)]
pub struct ServerMetrics {
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct ServerMetricsSnapshot {
    pub timestamp: String,
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
}

pub struct ServerMetricsHistory {
    buffer: RwLock<VecDeque<ServerMetricsSnapshot>>,
}

impl Default for ServerMetricsHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerMetricsHistory {
    pub fn new() -> Self {
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(SERVER_HISTORY_CAPACITY + 1)),
        }
    }

    pub async fn record(&self, snapshot: &ServerMetrics) -> ServerMetricsSnapshot {
        let snap = ServerMetricsSnapshot {
            timestamp: crate::db::models::now_iso8601(),
            cpu_percent: snapshot.cpu_percent,
            memory_used_bytes: snapshot.memory_used_bytes,
            memory_total_bytes: snapshot.memory_total_bytes,
            disk_used_bytes: snapshot.disk_used_bytes,
            disk_total_bytes: snapshot.disk_total_bytes,
        };
        let mut buf = self.buffer.write().await;
        buf.push_back(snap.clone());
        if buf.len() > SERVER_HISTORY_CAPACITY {
            buf.pop_front();
        }
        snap
    }

    pub async fn get_history(&self, limit: Option<usize>) -> Vec<ServerMetricsSnapshot> {
        let buf = self.buffer.read().await;
        match limit {
            Some(n) => buf.iter().rev().take(n).rev().cloned().collect(),
            None => buf.iter().cloned().collect(),
        }
    }
}

pub fn spawn_metrics_collector(
    metrics: Arc<RwLock<ServerMetrics>>,
    history: Arc<ServerMetricsHistory>,
    db: Arc<dyn Database>,
) {
    tokio::spawn(async move {
        let mut tick: u64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(COLLECT_INTERVAL_SECS)).await;

            let snapshot = tokio::task::spawn_blocking(|| {
                let mut sys = sysinfo::System::new();
                sys.refresh_cpu_all();
                sys.refresh_memory();
                std::thread::sleep(std::time::Duration::from_millis(200));
                sys.refresh_cpu_all();

                let disks = sysinfo::Disks::new_with_refreshed_list();
                let (disk_used, disk_total) =
                    disks.iter().fold((0u64, 0u64), |(used, total), disk| {
                        (
                            used + (disk.total_space() - disk.available_space()),
                            total + disk.total_space(),
                        )
                    });

                ServerMetrics {
                    cpu_percent: sys.global_cpu_usage(),
                    memory_used_bytes: sys.used_memory(),
                    memory_total_bytes: sys.total_memory(),
                    disk_used_bytes: disk_used,
                    disk_total_bytes: disk_total,
                }
            })
            .await;

            let snapshot = match snapshot {
                Ok(s) => s,
                Err(_) => continue,
            };

            let snap = history.record(&snapshot).await;
            *metrics.write().await = snapshot;

            tick += 1;

            if tick % SQLITE_WRITE_TICKS == 0 {
                if let Err(e) = db.insert_server_metric(&snap).await {
                    warn!("Failed to persist server metric: {e}");
                }
            }

            if tick % PRUNE_TICKS == 0 {
                let cutoff = chrono::Utc::now()
                    .checked_sub_signed(chrono::Duration::days(7))
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339();
                if let Err(e) = db.prune_server_metrics(&cutoff).await {
                    warn!("Failed to prune server metrics: {e}");
                }
            }
        }
    });
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/server/status", get(server_status))
        .route("/server/metrics/history", get(server_metrics_history))
        .route("/server/metrics/range", get(server_metrics_range))
}

async fn server_status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let metrics = state.server_metrics.read().await;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "cpu_percent": metrics.cpu_percent,
        "memory_used_bytes": metrics.memory_used_bytes,
        "memory_total_bytes": metrics.memory_total_bytes,
        "disk_used_bytes": metrics.disk_used_bytes,
        "disk_total_bytes": metrics.disk_total_bytes,
    })))
}

#[derive(Deserialize)]
struct HistoryParams {
    limit: Option<usize>,
}

async fn server_metrics_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit.map(|l| l.min(120));
    let data = state.server_metrics_history.get_history(limit).await;
    Ok(Json(serde_json::json!({ "data": data })))
}

#[derive(Deserialize)]
struct RangeParams {
    from: String,
    to: String,
    limit: Option<usize>,
}

async fn server_metrics_range(
    State(state): State<AppState>,
    Query(params): Query<RangeParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit.unwrap_or(500).min(2000);
    let data = state
        .db
        .query_server_metrics(&params.from, &params.to, limit)
        .await?;
    Ok(Json(
        serde_json::json!({ "data": data, "total": data.len() }),
    ))
}
