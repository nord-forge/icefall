use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewHealthCheck;
use crate::monitoring::health_runner::calculate_uptime;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps/{id}/health", get(get_health).put(update_health))
        .route("/health-checks/{id}", delete(delete_health_check))
}

#[derive(Deserialize)]
struct HealthQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

async fn get_health(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<HealthQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let checks = state.db.get_health_checks(&id).await?;
    let check_ids: Vec<String> = checks.iter().map(|c| c.id.clone()).collect();
    let all_events = state
        .db
        .get_health_events_for_checks(&check_ids, params.limit)
        .await?;

    let mut results = Vec::with_capacity(checks.len());
    for check in &checks {
        let events: Vec<_> = all_events
            .iter()
            .filter(|e| e.health_check_id == check.id)
            .cloned()
            .collect();
        let uptime = calculate_uptime(&events);
        let current_status = events.first().map_or("unknown", |e| e.status.as_str());

        results.push(serde_json::json!({
            "check": check,
            "current_status": current_status,
            "uptime_percent": uptime,
            "recent_events": events,
        }));
    }

    Ok(Json(serde_json::json!({ "data": results })))
}

#[derive(Deserialize)]
struct UpdateHealthRequest {
    check_type: Option<String>,
    interval_secs: Option<i64>,
    failure_threshold: Option<i64>,
    auto_restart: Option<bool>,
    config: Option<String>,
}

async fn update_health(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateHealthRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let checks = state.db.get_health_checks(&id).await?;

    if checks.is_empty() {
        let check = state
            .db
            .create_health_check(&NewHealthCheck {
                app_id: id,
                check_type: body.check_type.unwrap_or_else(|| "tcp".to_string()),
                config: body.config,
                interval_secs: body.interval_secs.unwrap_or(30),
                failure_threshold: body.failure_threshold.unwrap_or(3),
                auto_restart: body.auto_restart.unwrap_or(true),
            })
            .await?;
        Ok(Json(serde_json::json!({ "data": check })))
    } else {
        let check = &checks[0];
        state
            .db
            .update_health_check(
                &check.id,
                body.interval_secs,
                body.failure_threshold,
                body.auto_restart,
                body.config.as_deref(),
            )
            .await?;

        // Re-fetch updated checks to return current state
        let updated_checks = state.db.get_health_checks(&id).await?;
        let updated = updated_checks
            .first()
            .ok_or_else(|| ApiError::NotFound("health check not found after update".into()))?;

        Ok(Json(serde_json::json!({ "data": updated })))
    }
}

async fn delete_health_check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    state.db.delete_health_check(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}
