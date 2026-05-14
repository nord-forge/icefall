pub mod agent_ws;
pub mod apps;
pub mod audit;
pub mod auth;
pub mod backups;
pub mod clone;
pub mod databases;
pub mod db_browser;
pub mod db_restore;
pub mod db_ssl;
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
pub mod scheduled_tasks;
pub mod search;
pub mod server;
pub mod servers;
pub mod settings;
pub mod shared_variables;
pub mod terminal;
pub mod two_factor;
pub mod update;
pub mod users;
pub mod volumes;
pub mod webhook_endpoints;
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
        .merge(scheduled_tasks::routes())
        .merge(shared_variables::routes())
        .route("/search", axum::routing::get(search::search))
        .route("/apps/{id}/clone", axum::routing::post(clone::clone_app))
        .route("/apps/{id}/move", axum::routing::post(clone::move_app))
        .route(
            "/databases/{id}/restore",
            axum::routing::post(db_restore::restore_database),
        )
        .route(
            "/databases/{id}/restore/history",
            axum::routing::get(db_restore::list_restore_history),
        )
        .route(
            "/databases/{id}/ssl",
            axum::routing::put(db_ssl::update_database_ssl),
        )
        .route(
            "/databases/{id}/certificate",
            axum::routing::get(db_ssl::get_database_certificate),
        )
        .route(
            "/databases/{id}/certificate/regenerate",
            axum::routing::post(db_ssl::regenerate_certificate),
        )
        .route(
            "/notifications/webhooks",
            axum::routing::get(webhook_endpoints::list_endpoints)
                .post(webhook_endpoints::create_endpoint),
        )
        .route(
            "/notifications/webhooks/{id}",
            axum::routing::delete(webhook_endpoints::delete_endpoint),
        )
        .route(
            "/notifications/webhooks/{id}/deliveries",
            axum::routing::get(webhook_endpoints::list_deliveries),
        )
        .route(
            "/notifications/webhooks/{id}/test",
            axum::routing::post(webhook_endpoints::test_endpoint),
        )
}
