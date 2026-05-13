mod discovery;
mod operations;

use axum::routing::{get, post};
use axum::Router;

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps/{id}/volumes", get(operations::list_volumes))
        .route(
            "/apps/{id}/volumes/{mount_index}/browse",
            get(operations::browse_volume),
        )
        .route(
            "/apps/{id}/volumes/{mount_index}/download",
            get(operations::download_file),
        )
        .route(
            "/apps/{id}/volumes/{mount_index}/upload",
            post(operations::upload_file),
        )
        .route(
            "/apps/{id}/volumes/{mount_index}/size",
            get(operations::volume_size),
        )
        .route(
            "/apps/{id}/volumes/{mount_index}/delete",
            post(operations::delete_file),
        )
}
