use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CanaryResult {
    pub id: String,
    pub deploy_id: String,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub error_count: i32,
    pub total_requests: i32,
    pub verdict: String,
    pub created_at: String,
}
