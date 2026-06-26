mod operations;
mod query;

use axum::routing::{get, post};
use axum::Router;

use crate::api::AppState;

/// Shared trigger used by both the deploy route and the scheduled-deploy
/// scheduler (IF-179).
pub(crate) use operations::trigger_deploy;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/apps/{id}/deploys",
            get(query::list_deploys).post(operations::create_deploy),
        )
        .route("/apps/{id}/deploys/{deploy_id}", get(query::get_deploy))
        .route(
            "/apps/{id}/deploys/{deploy_id}/rollback",
            post(operations::rollback_deploy),
        )
        .route(
            "/deploys/{deploy_id}/cancel",
            post(operations::cancel_deploy),
        )
        .route(
            "/deploys/{deploy_id}/reschedule",
            post(operations::reschedule_deploy),
        )
        .route("/deploys/latest", get(query::get_latest_deploys))
}
