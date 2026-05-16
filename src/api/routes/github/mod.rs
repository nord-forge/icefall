mod setup;
mod webhook;

use axum::Router;

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(setup::routes())
        .merge(webhook::routes())
}
