mod crud;
mod detect;
mod drift;
mod insights;
mod lifecycle;
mod migrate;
mod proxy;
mod scaling;

use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps", get(crud::list_apps).post(crud::create_app))
        .route("/apps/detect", post(detect::detect_repo))
        .route("/apps/inactive", get(insights::list_inactive))
        .route("/apps/{id}/branches", get(insights::list_branches))
        .route(
            "/apps/{id}",
            get(crud::get_app)
                .put(crud::update_app)
                .delete(crud::delete_app),
        )
        .route("/apps/{id}/start", post(lifecycle::start_app))
        .route("/apps/{id}/stop", post(lifecycle::stop_app))
        .route("/apps/{id}/restart", post(lifecycle::restart_app))
        .route("/apps/{id}/wake", post(lifecycle::wake_app))
        .route("/apps/{id}/migrate", put(migrate::migrate_app))
        .route("/apps/{id}/drift", get(drift::check_drift))
        .route("/apps/{id}/scale", put(scaling::scale_app))
        .route("/apps/{id}/instances", get(scaling::list_instances))
        .route("/apps/{id}/lb-config", put(scaling::update_lb_config))
        .route(
            "/apps/{id}/instances/{instance_id}",
            delete(scaling::delete_instance),
        )
        // Reverse proxy management (IF-149)
        .route("/apps/{id}/proxy", get(proxy::get_proxy))
        .route("/apps/{id}/proxy/presets", put(proxy::update_presets))
        .route("/apps/{id}/proxy/custom", put(proxy::set_custom))
        .route("/apps/{id}/proxy/validate", post(proxy::validate))
        .route("/apps/{id}/proxy/reset", post(proxy::reset))
        .route("/apps/{id}/proxy/undo", post(proxy::undo))
        .route("/apps/{id}/proxy/history", get(proxy::history))
}
