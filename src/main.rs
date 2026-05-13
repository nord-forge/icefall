use clap::Parser;
use icefall::cli::{
    AppsCommands, Cli, Commands, DaemonCommands, DbCommands, DomainsCommands, EnvCommands,
    MigrateCommands, UpdateCommands,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { command } => match command {
            DaemonCommands::Start => icefall::cli::daemon::start().await,
            DaemonCommands::Stop => icefall::cli::daemon::stop().await,
            DaemonCommands::Status => icefall::cli::daemon::status().await,
        },
        Commands::Init => icefall::cli::init::run().await,
        Commands::Login => icefall::cli::commands::login::run().await,
        Commands::Deploy => icefall::cli::commands::deploy::run().await,
        Commands::Apps { command } => match command {
            AppsCommands::List => icefall::cli::commands::apps::list().await,
            AppsCommands::Info { app } => icefall::cli::commands::apps::info(&app).await,
        },
        Commands::Logs { app, search } => {
            icefall::cli::commands::logs::stream(&app, search.as_deref()).await;
        }
        Commands::Env { command } => match command {
            EnvCommands::Set { app, pair } => {
                icefall::cli::commands::env_vars::set(&app, &pair).await
            }
            EnvCommands::List { app } => icefall::cli::commands::env_vars::list(&app).await,
        },
        Commands::Domains { command } => match command {
            DomainsCommands::Add { app, domain } => {
                icefall::cli::commands::domains::add(&app, &domain).await;
            }
            DomainsCommands::List { app } => icefall::cli::commands::domains::list(&app).await,
        },
        Commands::Db { command } => match command {
            DbCommands::Create { db_type } => {
                icefall::cli::commands::databases::create(&db_type).await
            }
            DbCommands::List => icefall::cli::commands::databases::list().await,
            DbCommands::Backup { db } => icefall::cli::commands::databases::backup(&db).await,
        },
        Commands::Migrate { command } => match command {
            MigrateCommands::Export { output, dry_run } => {
                icefall::cli::commands::migrate::export(&output, dry_run).await;
            }
            MigrateCommands::Import { from, dry_run } => {
                icefall::cli::commands::migrate::import(&from, dry_run).await;
            }
        },
        Commands::Update {
            command,
            yes,
            channel,
            json,
        } => match command {
            None => icefall::cli::commands::update::run(yes, channel.as_deref(), json).await,
            Some(UpdateCommands::Check) => icefall::cli::commands::update::check(json).await,
            Some(UpdateCommands::Rollback { yes: rb_yes, check }) => {
                if check {
                    icefall::cli::commands::update::rollback_check().await
                } else {
                    icefall::cli::commands::update::rollback(yes || rb_yes).await
                }
            }
            Some(UpdateCommands::FromFile {
                file,
                manifest,
                signature,
            }) => {
                icefall::cli::commands::update::from_file(&file, &manifest, &signature, yes).await
            }
        },
        Commands::Status => icefall::cli::commands::server::status().await,
    }
}
