mod account;
mod admin;
mod helpers;
mod invitations;
mod tokens;

use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(admin::list_users))
        .route("/users/invite", post(invitations::invite_user))
        .route(
            "/users/me",
            get(account::get_me).delete(account::delete_own_account),
        )
        .route("/users/{id}/role", put(admin::change_role))
        .route("/users/{id}", delete(admin::deactivate_user))
        .route("/users/{id}/reset-password", post(admin::reset_password))
        .route("/users/{id}/2fa", delete(admin::admin_reset_2fa))
        .route("/users/accept-invite", post(invitations::accept_invite))
        .route(
            "/tokens",
            get(tokens::list_tokens).post(tokens::create_token),
        )
        .route("/tokens/{id}", delete(tokens::revoke_token))
}
