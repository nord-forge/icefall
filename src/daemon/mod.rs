pub mod health;
pub mod signals;
pub mod systemd;

use std::sync::Arc;

use listenfd::ListenFd;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use tokio::sync::RwLock;

use crate::api::routes::server::{
    spawn_metrics_collector as spawn_server_metrics, ServerMetrics, ServerMetricsHistory,
};
use crate::api::{self, AppState, BuildLockMap};
use crate::caddy::CaddyClient;
use crate::config::IcefallConfig;
use crate::db::encryption::Encryptor;
use crate::db::sqlite::SqliteDatabase;
use crate::db::Database;
use crate::docker::DockerClient;
use crate::events::EventBus;
use crate::monitoring::backup_scheduler::{spawn_backup_scheduler, BackupStore};
use crate::monitoring::health_runner::spawn_health_runner;
use crate::monitoring::instance_backup::{spawn_instance_backup_scheduler, InstanceBackupHandle};
use crate::monitoring::log_store::{spawn_log_capture, LogStore};
use crate::monitoring::metrics_collector::{
    spawn_metrics_collector as spawn_container_metrics, MetricsStore,
};

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),
    #[error("database error: {0}")]
    Database(#[from] crate::db::DbError),
    #[error("docker error: {0}")]
    Docker(#[from] crate::docker::DockerError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub struct DaemonRunner;

impl DaemonRunner {
    pub async fn start(config: IcefallConfig) -> Result<(), DaemonError> {
        // Set up tracing
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| config.log_level.parse().unwrap_or_default()),
            )
            .init();

        info!("Starting Icefall daemon v{}", env!("CARGO_PKG_VERSION"));

        // Validate config
        config.validate()?;

        // Write PID file
        let pid = std::process::id();
        if let Some(parent) = config.pid_file.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&config.pid_file, pid.to_string())
            .map_err(|e| {
                warn!(
                    "Could not write PID file to {}: {e}",
                    config.pid_file.display()
                );
                e
            })
            .ok();
        info!("PID {pid} written to {}", config.pid_file.display());

        // Create data directory
        std::fs::create_dir_all(&config.data_dir)?;

        // Decode encryption key
        let key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            config.encryption_key.as_deref().unwrap_or_default(),
        )
        .map_err(|e| DaemonError::Other(format!("Invalid encryption key: {e}")))?;
        let key: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| DaemonError::Other("Encryption key must be 32 bytes".to_string()))?;
        let encryptor = Arc::new(Encryptor::new(&key));

        // Connect to database
        let db_url = format!("sqlite:{}", config.sqlite_path.display());
        info!("Connecting to database at {}", config.sqlite_path.display());
        let db = SqliteDatabase::connect(&db_url, encryptor).await?;
        db.run_migrations().await?;
        info!("Database connected and migrations applied");
        let db: Arc<dyn Database> = Arc::new(db);

        // Connect to Docker
        let docker = match DockerClient::connect(&config.docker_socket).await {
            Ok(client) => {
                info!("Docker connection established");
                Arc::new(client)
            }
            Err(e) => {
                error!("Docker connection failed: {e}");
                return Err(DaemonError::Docker(e));
            }
        };

        // Connect to Caddy
        let caddy = Arc::new(CaddyClient::new(&config.caddy_admin_url));
        match caddy.health_check().await {
            Ok(()) => info!("Caddy connection established"),
            Err(e) => warn!("Caddy unreachable (will retry): {e}"),
        }

        // Run post-update health check (clears pending marker if successful)
        match crate::update::apply::post_update_check(&config.data_dir, db.as_ref(), &docker).await
        {
            Ok(()) => info!("Post-update check passed (or no pending update)"),
            Err(e) => warn!("Post-update check failed: {e}. Rollback may be triggered."),
        }

        // Create event bus, build locks, and monitoring stores
        let event_bus = Arc::new(EventBus::new(1024));
        let build_locks = Arc::new(BuildLockMap::new());
        let server_metrics = Arc::new(RwLock::new(ServerMetrics::default()));
        let server_metrics_history = Arc::new(ServerMetricsHistory::new());
        let metrics_store = Arc::new(MetricsStore::new());
        let log_store = Arc::new(LogStore::new(&config.data_dir));
        let backup_store = Arc::new(BackupStore::new(&config.data_dir));
        let instance_backup_handle = Arc::new(InstanceBackupHandle::new(db.clone()));

        // Start background tasks
        spawn_server_metrics(
            server_metrics.clone(),
            server_metrics_history.clone(),
            db.clone(),
        );
        spawn_health_runner(db.clone(), docker.clone(), event_bus.clone());
        spawn_container_metrics(
            docker.clone(),
            db.clone(),
            event_bus.clone(),
            metrics_store.clone(),
        );
        spawn_log_capture(docker.clone(), db.clone(), log_store.clone());
        spawn_backup_scheduler(docker.clone(), db.clone(), backup_store.clone());
        spawn_instance_backup_scheduler(db.clone(), instance_backup_handle.clone());

        // Spawn periodic update checker + auto-update applier
        crate::update::scheduler::spawn_update_checker(
            db.clone(),
            Arc::new(config.clone()),
            event_bus.clone(),
        );
        crate::update::scheduler::spawn_auto_update_applier(
            db.clone(),
            Arc::new(config.clone()),
            event_bus.clone(),
        );

        // Daily cleanup of old rollback binaries (7-day retention)
        {
            let data_dir = config.data_dir.clone();
            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
                loop {
                    interval.tick().await;
                    let rb = crate::update::rollback::UpdateRollback::new(&data_dir);
                    if let Err(e) = rb.cleanup_old_rollbacks(7) {
                        warn!(error = %e, "rollback cleanup failed");
                    }
                }
            });
        }

        let agent_registry = crate::agent::registry::AgentRegistry::new();

        // Build app state and router
        let state = AppState {
            db,
            docker,
            caddy,
            config: Arc::new(config.clone()),
            event_bus,
            build_locks,
            server_metrics,
            server_metrics_history,
            metrics_store,
            log_store,
            backup_store,
            instance_backup_handle,
            agent_registry,
        };

        crate::api::routes::agent_ws::spawn_heartbeat_checker(state.clone());

        let app = api::build_router(state);

        // Start HTTP server — prefer inherited socket from systemd, fall back to bind
        let listener = {
            let mut listenfd = ListenFd::from_env();
            if let Ok(Some(std_listener)) = listenfd.take_tcp_listener(0) {
                std_listener.set_nonblocking(true)?;
                let listener = TcpListener::from_std(std_listener)?;
                info!(
                    "API server listening via systemd socket activation on {:?}",
                    listener.local_addr()
                );
                listener
            } else {
                let addr = format!("{}:{}", config.listen_addr, config.listen_port);
                let listener = TcpListener::bind(&addr).await?;
                info!("API server listening on {addr}");
                listener
            }
        };

        // Signal systemd that we're ready
        if systemd::is_systemd_managed() {
            systemd::notify_ready();
            info!("Notified systemd: ready");

            // Watchdog ping every 30 seconds (WatchdogSec=60 in unit)
            tokio::spawn(async {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    systemd::notify_watchdog();
                }
            });
        }

        // Serve with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(signals::shutdown_signal())
            .await?;

        if systemd::is_systemd_managed() {
            systemd::notify_stopping();
        }

        info!("Daemon stopped");

        // Clean up PID file
        std::fs::remove_file(&config.pid_file).ok();

        Ok(())
    }
}
