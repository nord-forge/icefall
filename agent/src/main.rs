mod config;
mod connection;
mod context;
mod enroll;
mod handlers;

use std::collections::HashMap;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tokio::sync::Mutex;
use tracing::info;

use crate::context::HandlerContext;

const VERSION_LONG: &str = const_format::formatcp!(
    "{} ({} {} {})",
    env!("CARGO_PKG_VERSION"),
    env!("ICEFALL_AGENT_COMMIT"),
    env!("ICEFALL_AGENT_TARGET"),
    env!("ICEFALL_AGENT_BUILD_DATE"),
);

#[derive(Parser)]
#[command(name = "icefall-agent", about = "Icefall worker agent", long_version = VERSION_LONG)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to config file
    #[arg(long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Enroll this server with the Icefall control plane
    Enroll {
        /// Control plane URL
        #[arg(long)]
        url: String,
        /// Enrollment token
        #[arg(long)]
        token: String,
        /// Data directory for agent state
        #[arg(long, default_value = "/var/lib/icefall-agent")]
        data_dir: String,
    },
    /// Run the agent (connect to control plane and process commands)
    Run,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Enroll {
            url,
            token,
            data_dir,
        } => {
            tracing_subscriber::fmt().init();
            if let Err(e) = enroll::run_enrollment(&url, &token, &data_dir).await {
                eprintln!("Enrollment failed: {e}");
                std::process::exit(1);
            }
        }
        Commands::Run => {
            let cfg = match config::AgentConfig::load(cli.config.as_deref()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };

            let log_filter = cfg.log_level.clone();
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| log_filter.into()),
                )
                .init();

            info!("Icefall agent starting");
            info!("Control plane: {}", cfg.control_plane_url);
            info!("Server ID: {}", cfg.server_id);

            let docker = match bollard::Docker::connect_with_socket(
                &cfg.docker_socket,
                120,
                bollard::API_DEFAULT_VERSION,
            ) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to connect to Docker at {}: {e}", cfg.docker_socket);
                    std::process::exit(1);
                }
            };

            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

            let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();

            let ctx = HandlerContext {
                docker,
                caddy_url: cfg.caddy_admin_url.clone(),
                http: reqwest::Client::new(),
                event_tx,
                config: Arc::new(cfg),
                active_logs: Arc::new(Mutex::new(HashMap::new())),
                active_terminals: Arc::new(Mutex::new(HashMap::new())),
                shutdown: shutdown_rx.clone(),
            };

            tokio::spawn(async move {
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("Failed to register SIGTERM handler");
                let ctrl_c = tokio::signal::ctrl_c();

                tokio::select! {
                    _ = sigterm.recv() => info!("Received SIGTERM"),
                    _ = ctrl_c => info!("Received SIGINT"),
                }

                let _ = shutdown_tx.send(true);
            });

            // Spawn background metrics collector
            handlers::metrics::spawn_metrics_collector(ctx.clone());

            connection::run_connection_loop(ctx, shutdown_rx).await;
            info!("Agent shut down cleanly");
        }
    }
}
