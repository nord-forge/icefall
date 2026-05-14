use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

pub async fn search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if query.q.trim().is_empty() {
        return Ok(Json(serde_json::json!({ "data": {} })));
    }

    let results = state.db.search(&query.q).await?;
    Ok(Json(serde_json::json!({ "data": results })))
}
