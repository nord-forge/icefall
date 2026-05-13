use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps/{id}/logs", get(search_logs))
        .route("/apps/{id}/logs/download", get(download_logs))
}

#[derive(Deserialize)]
struct LogQuery {
    search: Option<String>,
    stream: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    200
}

async fn search_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<LogQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let results = state
        .log_store
        .search(
            &id,
            params.search.as_deref(),
            params.stream.as_deref(),
            params.limit,
        )
        .await;
    let count = results.len();

    Ok(Json(serde_json::json!({
        "data": results,
        "count": count,
    })))
}

async fn download_logs(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let content = state.log_store.read_all(&id).await;
    let filename = format!("attachment; filename=\"{id}-logs.txt\"");
    (
        [
            (
                header::CONTENT_TYPE,
                "text/plain; charset=utf-8".to_string(),
            ),
            (header::CONTENT_DISPOSITION, filename),
        ],
        content,
    )
}
