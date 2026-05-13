use bollard::models::{ContainerNetworkStats, ContainerStatsResponse};
use bollard::query_parameters::StatsOptions;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::docker::{DockerClient, DockerError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_limit_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

impl DockerClient {
    pub async fn get_stats(&self, container_id: &str) -> Result<ContainerStats, DockerError> {
        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = self.inner().stats(container_id, Some(options));

        let stats = stream
            .next()
            .await
            .ok_or_else(|| DockerError::ContainerNotFound(container_id.to_string()))??;

        Ok(parse_stats(&stats))
    }
}

fn parse_stats(stats: &ContainerStatsResponse) -> ContainerStats {
    let cpu_percent = calculate_cpu_percent(stats);

    let memory_usage_bytes = stats
        .memory_stats
        .as_ref()
        .and_then(|m| m.usage)
        .unwrap_or(0);

    let memory_limit_bytes = stats
        .memory_stats
        .as_ref()
        .and_then(|m| m.limit)
        .unwrap_or(0);

    let (network_rx_bytes, network_tx_bytes) = stats
        .networks
        .as_ref()
        .map_or(
            (0, 0), |nets: &std::collections::HashMap<String, ContainerNetworkStats>| {
                nets.values().fold((0u64, 0u64), |(rx, tx), net| {
                    (
                        rx + net.rx_bytes.unwrap_or(0),
                        tx + net.tx_bytes.unwrap_or(0),
                    )
                })
            },
        );

    ContainerStats {
        cpu_percent,
        memory_usage_bytes,
        memory_limit_bytes,
        network_rx_bytes,
        network_tx_bytes,
    }
}

fn calculate_cpu_percent(stats: &ContainerStatsResponse) -> f64 {
    let total_usage = stats
        .cpu_stats
        .as_ref()
        .and_then(|c| c.cpu_usage.as_ref())
        .and_then(|u| u.total_usage)
        .unwrap_or(0);

    let pre_total_usage = stats
        .precpu_stats
        .as_ref()
        .and_then(|c| c.cpu_usage.as_ref())
        .and_then(|u| u.total_usage)
        .unwrap_or(0);

    let cpu_delta = total_usage as f64 - pre_total_usage as f64;

    let system_cpu = stats
        .cpu_stats
        .as_ref()
        .and_then(|c| c.system_cpu_usage)
        .unwrap_or(0);

    let pre_system_cpu = stats
        .precpu_stats
        .as_ref()
        .and_then(|c| c.system_cpu_usage)
        .unwrap_or(0);

    let system_delta = system_cpu as f64 - pre_system_cpu as f64;

    let num_cpus = stats
        .cpu_stats
        .as_ref()
        .and_then(|c| c.online_cpus)
        .unwrap_or(1) as f64;

    if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta) * num_cpus * 100.0
    } else {
        0.0
    }
}
