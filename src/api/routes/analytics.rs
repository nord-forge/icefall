use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    from: Option<String>,
    to: Option<String>,
    days: Option<i64>,
}

pub async fn deploy_analytics(
    State(state): State<AppState>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let to = query.to.unwrap_or_else(crate::db::models::now_iso8601);
    let from = query.from.unwrap_or_else(|| {
        let days = query.days.unwrap_or(30);
        (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339()
    });

    let analytics = state.db.get_deploy_analytics(&from, &to).await?;
    Ok(Json(serde_json::json!({ "data": analytics })))
}
