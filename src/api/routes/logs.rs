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
    /// Suppress lines matching the app's noise patterns (IF-193). Default true.
    #[serde(default = "default_true")]
    suppress_noise: bool,
}

fn default_limit() -> usize {
    200
}

fn default_true() -> bool {
    true
}

/// Split a stored pattern blob (one pattern per line) into lowercased,
/// non-empty substrings used for case-insensitive matching.
fn parse_patterns(blob: &Option<String>) -> Vec<String> {
    blob.as_deref()
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_lowercase())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
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

    // Noise suppression + anomaly highlighting from the app's stored patterns.
    let app = state.db.get_app(&id).await?;
    let noise = app
        .as_ref()
        .map(|a| parse_patterns(&a.log_noise_patterns))
        .unwrap_or_default();
    let highlight = app
        .as_ref()
        .map(|a| parse_patterns(&a.log_highlight_patterns))
        .unwrap_or_default();

    let mut suppressed = 0usize;
    let data: Vec<serde_json::Value> = results
        .into_iter()
        .filter_map(|line| {
            let lower = line.message.to_lowercase();
            if params.suppress_noise && noise.iter().any(|p| lower.contains(p)) {
                suppressed += 1;
                return None;
            }
            let highlighted = highlight.iter().any(|p| lower.contains(p));
            Some(serde_json::json!({
                "timestamp": line.timestamp,
                "stream": line.stream,
                "message": line.message,
                "highlighted": highlighted,
            }))
        })
        .collect();
    let count = data.len();

    Ok(Json(serde_json::json!({
        "data": data,
        "count": count,
        "suppressed_count": suppressed,
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
