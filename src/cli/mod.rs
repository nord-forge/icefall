pub mod client;
pub mod commands;
pub mod daemon;
pub mod init;

use clap::{Parser, Subcommand};

const fn build_version() -> &'static str {
    concat!(
        env!("CARGO_PKG_VERSION"),
        " (",
        env!("ICEFALL_GIT_COMMIT"),
        " ",
        env!("ICEFALL_TARGET_TRIPLE"),
        " ",
        env!("ICEFALL_BUILD_DATE"),
        ")"
    )
}

#[derive(Parser)]
#[command(
    name = "icefall",
    version = build_version(),
    about = "A fast, simple, self-hosted deployment platform"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage the icefall daemon
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
    /// Initialize icefall configuration
    Init,
    /// Authenticate with the icefall server
    Login,
    /// Deploy the current project
    Deploy,
    /// Manage applications
    Apps {
        #[command(subcommand)]
        command: AppsCommands,
    },
    /// Stream application logs
    Logs {
        /// Application name
        app: String,
        /// Search filter
        #[arg(long)]
        search: Option<String>,
    },
    /// Manage environment variables
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },
    /// Manage custom domains
    Domains {
        #[command(subcommand)]
        command: DomainsCommands,
    },
    /// Manage databases
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    /// Server migration (export/import)
    Migrate {
        #[command(subcommand)]
        command: MigrateCommands,
    },
    /// Self-update icefall
    Update {
        #[command(subcommand)]
        command: Option<UpdateCommands>,

        /// Skip confirmation prompt
        #[arg(long, global = true)]
        yes: bool,

        /// Override the update channel (stable, beta)
        #[arg(long, global = true)]
        channel: Option<String>,

        /// Output structured JSON per step (for scripting)
        #[arg(long, global = true)]
        json: bool,
    },
    /// Show server status overview
    Status,
}

#[derive(Subcommand)]
pub enum UpdateCommands {
    /// Check for updates without applying
    Check,
    /// Roll back to the previous version
    Rollback {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Automated check for ExecStopPost — only rolls back if pending update marker is recent
        #[arg(long)]
        check: bool,
    },
    /// Update from a local file (air-gapped / offline)
    FromFile {
        /// Path to the tarball
        file: String,
        /// Path to the release manifest
        #[arg(long)]
        manifest: String,
        /// Path to the manifest signature
        #[arg(long)]
        signature: String,
    },
}

#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon (foreground)
    Start,
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
}

#[derive(Subcommand)]
pub enum AppsCommands {
    /// List all applications
    List,
    /// Show application details
    Info {
        /// Application name
        app: String,
    },
}

#[derive(Subcommand)]
pub enum EnvCommands {
    /// Set an environment variable
    Set {
        /// Application name
        app: String,
        /// KEY=value pair
        pair: String,
    },
    /// List environment variables
    List {
        /// Application name
        app: String,
    },
}

#[derive(Subcommand)]
pub enum DomainsCommands {
    /// Add a custom domain
    Add {
        /// Application name
        app: String,
        /// Domain name
        domain: String,
    },
    /// List domains for an application
    List {
        /// Application name
        app: String,
    },
}

#[derive(Subcommand)]
pub enum DbCommands {
    /// Create a managed database
    Create {
        /// Database type (postgres, mysql, redis, mongo)
        db_type: String,
    },
    /// List managed databases
    List,
    /// Trigger a manual backup
    Backup {
        /// Database name
        db: String,
    },
}

#[derive(Subcommand)]
pub enum MigrateCommands {
    /// Export server state
    Export {
        /// Output file path (local path or s3://bucket/key)
        #[arg(long, default_value = "icefall-backup.tar.gz")]
        output: String,
        /// Show what would be exported without creating the archive
        #[arg(long)]
        dry_run: bool,
    },
    /// Import server state
    Import {
        /// Input file path (local path or s3://bucket/key)
        #[arg(long)]
        from: String,
        /// Show what the archive contains without restoring
        #[arg(long)]
        dry_run: bool,
    },
}
