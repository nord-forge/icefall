mod channels;
pub mod dispatch;
pub mod emit;
mod rules;

use axum::routing::{delete, get, post};
use axum::Router;

use crate::api::AppState;

pub use dispatch::dispatch_notification;
pub use emit::emit_event;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/notifications/channels",
            get(channels::list_channels).post(channels::create_channel),
        )
        .route(
            "/notifications/channels/{id}",
            delete(channels::delete_channel),
        )
        .route(
            "/notifications/channels/{id}/test",
            post(channels::test_channel),
        )
        .route(
            "/notifications/global-rules",
            get(rules::list_global_rules).post(rules::create_global_rule),
        )
        .route(
            "/apps/{app_id}/notifications",
            get(rules::list_rules).post(rules::create_rule),
        )
        .route(
            "/apps/{app_id}/notifications/{rule_id}",
            delete(rules::delete_rule),
        )
}
