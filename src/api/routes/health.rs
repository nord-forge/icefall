use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewHealthCheck;
use crate::monitoring::health_runner::calculate_uptime;

pub fn routes() -> Router<AppState> {
    Router::new().route("/apps/{id}/health", get(get_health).put(update_health))
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
        Ok(Json(
            serde_json::json!({ "data": checks[0], "note": "update existing health checks not yet supported — delete and recreate" }),
        ))
    }
}
