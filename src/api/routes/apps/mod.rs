mod crud;
mod drift;
mod lifecycle;
mod migrate;
mod scaling;

use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps", get(crud::list_apps).post(crud::create_app))
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
}
