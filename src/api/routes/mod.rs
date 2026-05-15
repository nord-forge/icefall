pub mod agent_ws;
pub mod analytics;
pub mod apps;
pub mod audit;
pub mod auth;
pub mod backups;
pub mod bundles;
pub mod cleanup;
pub mod config_history;
pub mod databases;
pub mod db_browser;
pub mod deploys;
pub mod domains;
pub mod env_vars;
pub mod environments;
pub mod events;
pub mod forecast;
pub mod git_sources;
pub mod github;
pub mod health;
pub mod incidents;
pub mod instance_backup;
pub mod log_drains;
pub mod logs;
pub mod mcp;
pub mod metrics;
pub mod notifications;
pub mod oauth;
pub mod onboarding;
pub mod openapi;
pub mod profile;
pub mod projects;
pub mod search;
pub mod server;
pub mod servers;
pub mod settings;
pub mod shared_variables;
pub mod teams;
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
        .merge(incidents::routes())
        .merge(terminal::routes())
        .merge(two_factor::routes())
        .merge(oauth::routes())
        .merge(projects::routes())
        .merge(volumes::routes())
        .merge(update::routes())
        .merge(audit::routes())
        .merge(environments::routes())
        .merge(log_drains::routes())
        .merge(git_sources::routes())
        .merge(github::routes())
        .merge(cleanup::routes())
        .merge(teams::routes())
        .merge(shared_variables::routes())
        .merge(openapi::routes())
        .route("/search", axum::routing::get(search::search))
        .route(
            "/analytics/deploys",
            axum::routing::get(analytics::deploy_analytics),
        )
        .route(
            "/apps/{id}/config-history",
            axum::routing::get(config_history::list_app_config_history),
        )
        .route(
            "/deploys/{id}/events",
            axum::routing::get(config_history::list_deploy_events),
        )
        .route(
            "/deploys/{id}/approve",
            axum::routing::post(config_history::approve_deploy),
        )
        .route(
            "/servers/{id}/forecast",
            axum::routing::get(forecast::server_forecast),
        )
        .route("/templates", axum::routing::get(list_templates))
        .route(
            "/apps/{id}/export",
            axum::routing::get(bundles::export_bundle),
        )
        .route(
            "/bundles/import",
            axum::routing::post(bundles::import_bundle),
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

async fn list_templates(
    axum::extract::State(state): axum::extract::State<crate::api::AppState>,
) -> Result<axum::Json<serde_json::Value>, crate::api::error::ApiError> {
    let templates = state.db.list_service_templates().await?;
    Ok(axum::Json(serde_json::json!({ "data": templates })))
}
