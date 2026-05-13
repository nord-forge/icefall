pub mod agent_ws;
pub mod apps;
pub mod audit;
pub mod auth;
pub mod backups;
pub mod databases;
pub mod db_browser;
pub mod deploys;
pub mod domains;
pub mod env_vars;
pub mod events;
pub mod health;
pub mod instance_backup;
pub mod logs;
pub mod mcp;
pub mod metrics;
pub mod notifications;
pub mod oauth;
pub mod onboarding;
pub mod openapi;
pub mod profile;
pub mod projects;
pub mod server;
pub mod servers;
pub mod settings;
pub mod terminal;
pub mod two_factor;
pub mod update;
pub mod users;
pub mod volumes;
pub mod webhooks;

use axum::Router;

use crate::api::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(agent_ws::routes())
        .merge(apps::routes())
        .merge(auth::routes())
        .merge(backups::routes())
        .merge(databases::routes())
        .merge(db_browser::routes())
        .merge(deploys::routes())
        .merge(domains::routes())
        .merge(env_vars::routes())
        .merge(health::routes())
        .merge(logs::routes())
        .merge(metrics::routes())
        .merge(profile::routes())
        .merge(users::routes())
        .merge(settings::routes())
        .merge(server::routes())
        .merge(servers::routes())
        .merge(events::routes())
        .merge(webhooks::routes())
        .merge(notifications::routes())
        .merge(onboarding::routes())
        .merge(mcp::routes())
        .merge(instance_backup::routes())
        .merge(terminal::routes())
        .merge(two_factor::routes())
        .merge(oauth::routes())
        .merge(projects::routes())
        .merge(volumes::routes())
        .merge(update::routes())
        .merge(audit::routes())
        .merge(openapi::routes())
}
