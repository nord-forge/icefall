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
use crate::db::models::CONTROL_PLANE_SERVER_ID;
use crate::db::Database;
use crate::events::{EventBus, EventType};

const SERVER_HISTORY_CAPACITY: usize = 120;
const COLLECT_INTERVAL_SECS: u64 = 2;
const SQLITE_WRITE_TICKS: u64 = 15; // every 30s (15 * 2s)
const PRUNE_TICKS: u64 = 1800; // every hour (1800 * 2s)
const DISK_REFRESH_TICKS: u64 = 15; // re-enumerate disks every 30s (15 * 2s)

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
    event_bus: Arc<EventBus>,
    caddy_admin_url: String,
) {
    tokio::spawn(async move {
        let mut tick: u64 = 0;
        // Persistent sysinfo handles, reused across cycles (IF-260): CPU% comes
        // from the delta between cycles instead of an in-cycle sleep or reparse.
        let mut sys = sysinfo::System::new();
        let mut disks = sysinfo::Disks::new_with_refreshed_list();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(COLLECT_INTERVAL_SECS)).await;

            // Disk topology rarely changes — only re-enumerate it occasionally
            // rather than every cycle.
            let refresh_disks = tick % DISK_REFRESH_TICKS == 0;

            // Move the persistent handles into the blocking refresh and take them
            // back, so they survive across iterations without crossing an .await.
            let result = tokio::task::spawn_blocking(move || {
                sys.refresh_cpu_all();
                sys.refresh_memory();
                if refresh_disks {
                    disks.refresh(true);
                }

                // Report the root filesystem only. Summing every mount
                // double-counts the same underlying device (container overlay
                // mounts, bind mounts, tmpfs) and inflates totals — and worse,
                // the number drifts as containers come and go. Pick the disk
                // mounted at `/`; if absent, fall back to the largest device.
                let root_disk = disks
                    .iter()
                    .find(|d| d.mount_point() == std::path::Path::new("/"))
                    .or_else(|| disks.iter().max_by_key(|d| d.total_space()));
                let (disk_used, disk_total) = root_disk
                    .map(|d| (d.total_space() - d.available_space(), d.total_space()))
                    .unwrap_or((0, 0));

                let snapshot = ServerMetrics {
                    // First cycle has no prior CPU sample, so this reads 0%; it's
                    // accurate from the second cycle on.
                    cpu_percent: sys.global_cpu_usage(),
                    memory_used_bytes: sys.used_memory(),
                    memory_total_bytes: sys.total_memory(),
                    disk_used_bytes: disk_used,
                    disk_total_bytes: disk_total,
                };
                (snapshot, sys, disks)
            })
            .await;

            let Ok((snapshot, returned_sys, returned_disks)) = result else {
                // The blocking task panicked; the handles are gone. Recreate them
                // so the loop can continue.
                sys = sysinfo::System::new();
                disks = sysinfo::Disks::new_with_refreshed_list();
                continue;
            };
            sys = returned_sys;
            disks = returned_disks;

            let snap = history.record(&snapshot).await;
            *metrics.write().await = snapshot.clone();

            // IF-216: Disk usage alert evaluation
            if tick % 30 == 0 {
                if let Ok(Some(server)) = db.get_server(CONTROL_PLANE_SERVER_ID).await {
                    if server.disk_alert_enabled && snapshot.disk_total_bytes > 0 {
                        let usage_pct = (snapshot.disk_used_bytes as f64
                            / snapshot.disk_total_bytes as f64
                            * 100.0) as i32;
                        let prev_state = server.disk_alert_state.as_str();

                        let new_state = if usage_pct >= server.disk_alert_critical_threshold {
                            "critical"
                        } else if usage_pct >= server.disk_alert_warning_threshold {
                            "warning"
                        } else {
                            "normal"
                        };

                        if new_state != prev_state {
                            let event_name = match (prev_state, new_state) {
                                (_, "critical") => Some("server.disk.critical"),
                                (_, "warning") if prev_state == "normal" => {
                                    Some("server.disk.warning")
                                }
                                ("warning" | "critical", "normal") => Some("server.disk.recovered"),
                                _ => None,
                            };

                            if let Some(event) = event_name {
                                let threshold = if new_state == "critical" {
                                    server.disk_alert_critical_threshold
                                } else {
                                    server.disk_alert_warning_threshold
                                };
                                let details = serde_json::json!({
                                    "event": event,
                                    "server": server.name,
                                    "disk_usage_percent": usage_pct,
                                    "threshold": threshold,
                                });
                                event_bus.emit(
                                    EventType::DiskAlert,
                                    None,
                                    Some(CONTROL_PLANE_SERVER_ID),
                                    details.clone(),
                                );

                                // IF-167: disk threshold notification. The state
                                // machine only fires on transition (re-alert cooldown).
                                crate::api::routes::notifications::emit_event(
                                    &db,
                                    &caddy_admin_url,
                                    event,
                                    None,
                                    &format!("Disk on '{}' at {usage_pct}% ({event})", server.name),
                                    details,
                                )
                                .await;
                            }

                            let _ = db
                                .update_server_disk_alert_state(CONTROL_PLANE_SERVER_ID, new_state)
                                .await;
                        }
                    }
                }
            }

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
