mod crud;
mod lifecycle;
mod migrate;

use axum::routing::{get, post, put};
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
        .route("/apps/{id}/migrate", put(migrate::migrate_app))
}
