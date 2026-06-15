use serde::{Deserialize, Serialize};

/// A single persisted per-container metrics sample (IF-191). Recorded by the
/// metrics collector so the resource packer can analyze multi-day usage.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContainerMetricsRecord {
    pub id: String,
    pub app_id: String,
    pub cpu_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_limit_bytes: i64,
    pub recorded_at: String,
}

/// Insert payload for a container metrics sample.
pub struct NewContainerMetricsRecord {
    pub app_id: String,
    pub cpu_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_limit_bytes: i64,
}

/// Aggregated per-app usage over an analysis window, the input to right-sizing.
/// `sample_count` lets the engine ignore apps with too little data to trust.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContainerUsageStats {
    pub app_id: String,
    pub avg_cpu_percent: f64,
    pub peak_cpu_percent: f64,
    pub avg_memory_bytes: i64,
    pub peak_memory_bytes: i64,
    /// Most recent memory limit observed for the container (0 = unlimited).
    pub memory_limit_bytes: i64,
    pub sample_count: i64,
}
