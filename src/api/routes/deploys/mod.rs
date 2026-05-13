mod operations;
mod query;

use axum::routing::{get, post};
use axum::Router;

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/apps/{id}/deploys",
            get(query::list_deploys).post(operations::create_deploy),
        )
        .route(
            "/apps/{id}/deploys/{deploy_id}/rollback",
            post(operations::rollback_deploy),
        )
        .route("/deploys/latest", get(query::get_latest_deploys))
}
