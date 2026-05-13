use std::time::Duration;

use bollard::query_parameters::ListContainersOptions;
use futures_util::StreamExt;
use icefall_common::protocol::AgentMessage;
use sysinfo::{Disks, System};
use tracing::{debug, warn};

use crate::context::HandlerContext;

const METRICS_INTERVAL: Duration = Duration::from_secs(10);

pub fn spawn_metrics_collector(ctx: HandlerContext) {
    tokio::spawn(async move {
        let mut shutdown = ctx.shutdown.clone();
        let mut interval = tokio::time::interval(METRICS_INTERVAL);
        let mut sys = System::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    collect_and_send(&ctx, &mut sys).await;
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        debug!("metrics collector shutting down");
                        break;
                    }
                }
            }
        }
    });
}

async fn collect_and_send(ctx: &HandlerContext, sys: &mut System) {
    // System metrics
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpu_percent = sys.global_cpu_usage() as f64;
    let ram_used = sys.used_memory() as i64;
    let ram_total = sys.total_memory() as i64;

    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks.iter().fold((0i64, 0i64), |(used, total), d| {
        (
            used + (d.total_space() - d.available_space()) as i64,
            total + d.total_space() as i64,
        )
    });

    let load_avg = System::load_average();

    let system_data = serde_json::json!({
        "cpu_percent": cpu_percent,
        "ram_used_bytes": ram_used,
        "ram_total_bytes": ram_total,
        "disk_used_bytes": disk_used,
        "disk_total_bytes": disk_total,
        "load_average": [load_avg.one, load_avg.five, load_avg.fifteen],
    });

    let _ = ctx.event_tx.send(AgentMessage::Event {
        event_type: "metrics.system".to_string(),
        data: system_data,
    });

    // Per-container metrics
    let options = ListContainersOptions {
        all: false,
        ..Default::default()
    };

    let containers = match ctx.docker.list_containers(Some(options)).await {
        Ok(c) => c,
        Err(e) => {
            warn!("failed to list containers for metrics: {e}");
            return;
        }
    };

    for container in containers {
        let id = match &container.id {
            Some(id) => id.clone(),
            None => continue,
        };

        let short_id = &id[..12.min(id.len())];

        let stats_opts = bollard::query_parameters::StatsOptions {
            stream: false,
            one_shot: true,
        };
        let mut stats_stream = ctx.docker.stats(&id, Some(stats_opts));
        if let Some(Ok(stats)) = stats_stream.next().await {
            let (cpu_pct, mem_usage, mem_limit) = {
                let cpu_stats = stats.cpu_stats.as_ref();
                let precpu_stats = stats.precpu_stats.as_ref();

                let cpu_delta = cpu_stats
                    .and_then(|c| c.cpu_usage.as_ref())
                    .and_then(|u| u.total_usage)
                    .unwrap_or(0) as f64
                    - precpu_stats
                        .and_then(|c| c.cpu_usage.as_ref())
                        .and_then(|u| u.total_usage)
                        .unwrap_or(0) as f64;

                let system_delta = cpu_stats.and_then(|c| c.system_cpu_usage).unwrap_or(0) as f64
                    - precpu_stats.and_then(|c| c.system_cpu_usage).unwrap_or(0) as f64;

                let num_cpus = cpu_stats.and_then(|c| c.online_cpus).unwrap_or(1);

                let pct = if system_delta > 0.0 {
                    (cpu_delta / system_delta) * num_cpus as f64 * 100.0
                } else {
                    0.0
                };

                let mem_stats = stats.memory_stats.as_ref();
                let usage = mem_stats.and_then(|m| m.usage).unwrap_or(0);
                let limit = mem_stats.and_then(|m| m.limit).unwrap_or(0);

                (pct, usage, limit)
            };

            let _ = ctx.event_tx.send(AgentMessage::Event {
                event_type: "metrics.container".to_string(),
                data: serde_json::json!({
                    "container_id": id,
                    "name": container.names.as_ref().and_then(|n| n.first()).map(|n| n.trim_start_matches('/')),
                    "cpu_percent": cpu_pct,
                    "memory_bytes": mem_usage,
                    "memory_limit_bytes": mem_limit,
                }),
            });

            debug!(id = short_id, cpu = %format!("{cpu_pct:.1}%"), mem_mb = mem_usage / 1_048_576, "container metrics");
        }
    }
}
