mod auth_flow;
mod management;
mod pkce;
mod providers;

use axum::routing::{delete, get};
use axum::Router;

use crate::api::AppState;

use auth_flow::{oauth_authorize, oauth_callback, oauth_link};
use management::{
    get_enabled_providers, get_oauth_settings, list_oauth_identities, oauth_unlink,
    update_oauth_settings,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/oauth/{provider}/authorize", get(oauth_authorize))
        .route("/auth/oauth/{provider}/callback", get(oauth_callback))
        .route("/auth/oauth/{provider}/link", get(oauth_link))
        .route("/auth/oauth/{provider}/unlink", delete(oauth_unlink))
        .route("/auth/oauth/identities", get(list_oauth_identities))
        .route(
            "/settings/oauth",
            get(get_oauth_settings).put(update_oauth_settings),
        )
        .route("/settings/oauth/providers", get(get_enabled_providers))
}
