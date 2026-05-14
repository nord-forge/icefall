pub mod error;
pub mod middleware;
pub mod routes;
pub mod utils;

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use tokio::sync::{Mutex, RwLock};
use tower_http::services::ServeDir;

use crate::agent::registry::AgentRegistry;
use crate::api::routes::server::{ServerMetrics, ServerMetricsHistory};
use crate::caddy::CaddyClient;
use crate::config::IcefallConfig;
use crate::db::Database;
use crate::docker::DockerClient;
use crate::events::EventBus;
use crate::monitoring::backup_scheduler::BackupStore;
use crate::monitoring::instance_backup::InstanceBackupHandle;
use crate::monitoring::log_store::LogStore;
use crate::monitoring::metrics_collector::MetricsStore;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<dyn Database>,
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
    pub config: Arc<IcefallConfig>,
    pub event_bus: Arc<EventBus>,
    pub build_locks: Arc<BuildLockMap>,
    pub server_metrics: Arc<RwLock<ServerMetrics>>,
    pub server_metrics_history: Arc<ServerMetricsHistory>,
    pub metrics_store: Arc<MetricsStore>,
    pub log_store: Arc<LogStore>,
    pub backup_store: Arc<BackupStore>,
    pub instance_backup_handle: Arc<InstanceBackupHandle>,
    pub agent_registry: Arc<AgentRegistry>,
}

pub struct BuildLockMap {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl Default for BuildLockMap {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildLockMap {
    pub fn new() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn acquire(&self, app_id: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry(app_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

pub fn build_router(state: AppState) -> Router {
    let api_routes = routes::api_routes();

    let router = Router::new()
        .nest("/api/v1", api_routes)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ))
        .fallback_service(ServeDir::new("dashboard/dist").append_index_html_on_directories(true));

    middleware::apply_middleware(router, &state.config).with_state(state)
}
