mod agent;
mod crud;
mod scripts;

use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::Router;
use sha2::Digest;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/servers", get(crud::list_servers).post(crud::create_server))
        .route("/servers/setup", get(scripts::setup_script))
        .route(
            "/servers/{id}",
            get(crud::get_server)
                .put(crud::update_server)
                .delete(crud::delete_server),
        )
        .route("/servers/{id}/token", post(crud::regenerate_token))
        .route("/servers/{id}/update", post(agent::update_agent))
        .route("/servers/update-all", post(agent::update_all_agents))
        .route("/agent/download/{target}", get(scripts::download_agent))
        .route("/agent/uninstall", get(scripts::uninstall_script))
}

pub(crate) async fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let user = authenticate_from_headers(state, headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if user.role != "admin" {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    Ok(())
}

pub(crate) fn generate_enrollment_token() -> (String, String) {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use rand::Rng;

    let random_bytes: [u8; 32] = rand::rng().random();
    let token = URL_SAFE_NO_PAD.encode(random_bytes);

    let mut hasher = sha2::Sha256::new();
    hasher.update(token.as_bytes());
    let hash = hex::encode(hasher.finalize());

    (token, hash)
}
