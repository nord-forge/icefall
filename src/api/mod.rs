pub mod error;
pub mod middleware;
pub mod rate_limit;
pub mod routes;
pub mod utils;

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::response::IntoResponse;
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

/// 1 MiB cap on request bodies (audit H10). Generous for JSON API payloads;
/// upload-style routes can raise it locally with their own `DefaultBodyLimit`.
const MAX_BODY_BYTES: usize = 1024 * 1024;

const DASHBOARD_DIST: &str = "dashboard/dist";

/// Dynamic dashboard routes. The Astro build is fully static (no SSR), so
/// each dynamic route is prerendered to a placeholder shell at
/// `dist/<prefix>/_/index.html`; the client island reads the real id from
/// `window.location`. When `ServeDir` cannot find a file for a request,
/// this maps the URL prefix to the matching shell so e.g. `/teams/abc123`
/// serves `dist/teams/_/index.html`, not the wrong page.
const DYNAMIC_ROUTE_PREFIXES: &[&str] = &["/teams/", "/servers/", "/apps/", "/invitations/"];

/// SPA fallback handler: pick the prerendered shell for an unmatched path.
async fn dashboard_fallback(uri: axum::http::Uri) -> axum::response::Response {
    let path = uri.path();
    let shell = DYNAMIC_ROUTE_PREFIXES
        .iter()
        .find(|p| path.starts_with(*p))
        .map_or_else(
            || format!("{DASHBOARD_DIST}/index.html"),
            |prefix| format!("{DASHBOARD_DIST}{prefix}_/index.html"),
        );

    match tokio::fs::read(&shell).await {
        Ok(bytes) => (
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            bytes,
        )
            .into_response(),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "dashboard not built").into_response(),
    }
}

pub fn build_router(state: AppState) -> Router {
    let api_routes = routes::api_routes();

    let serve_dir = ServeDir::new(DASHBOARD_DIST)
        .append_index_html_on_directories(true)
        .fallback(axum::routing::get(dashboard_fallback));

    let router = Router::new()
        .nest("/api/v1", api_routes)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .fallback_service(serve_dir);

    middleware::apply_middleware(router, &state.config).with_state(state)
}
