mod config;
mod crud;
mod lifecycle;
mod linking;
mod public_access;
pub(crate) mod readonly;

/// The fixed username of the read-only db_browser account (see `config`).
pub(crate) use config::READONLY_USER;

use axum::routing::{delete, get, post};
use axum::Router;

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/databases",
            get(crud::list_databases).post(crud::create_database),
        )
        .route(
            "/databases/{id}",
            get(crud::get_database).delete(crud::delete_database),
        )
        .route("/databases/{id}/link/{app_id}", post(linking::link_to_app))
        .route(
            "/databases/{id}/link/{app_id}",
            delete(linking::unlink_from_app),
        )
        .route("/databases/{id}/start", post(lifecycle::start_database))
        .route("/databases/{id}/stop", post(lifecycle::stop_database))
        .route("/databases/{id}/restart", post(lifecycle::restart_database))
        // Public TCP access (IF-172): GET status, POST enable, DELETE disable.
        .route(
            "/databases/{id}/public-access",
            get(public_access::get_public_access)
                .post(public_access::enable_public_access)
                .delete(public_access::disable_public_access),
        )
}
