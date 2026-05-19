mod discovery;
mod operations;
mod preferences;

use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::Router;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

const DEFAULT_GITHUB_REPO: &str = "nordforge/icefall";

async fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let user = authenticate_from_headers(state, headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if user.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }
    Ok(())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/system/update/check", get(discovery::check_for_update))
        .route("/system/update/status", get(discovery::get_update_status))
        .route("/system/update/download", post(operations::start_download))
        .route("/system/update/apply", post(operations::apply_update))
        .route("/system/update/rollback", post(operations::rollback_update))
        .route("/system/update/skip", post(operations::skip_version))
        .route(
            "/system/update/preferences",
            get(preferences::get_update_preferences).patch(preferences::update_preferences),
        )
        .route("/system/update/history", get(discovery::get_update_history))
}
