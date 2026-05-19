use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::db::models::NewLogDrain;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps/{id}/log-drains", get(list_drains).post(create_drain))
        .route("/log-drains/{id}", put(update_drain).delete(delete_drain))
        .route("/log-drains/{id}/test", post(test_drain))
        .route("/log-drains", get(list_global_drains))
}

async fn list_drains(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Read-only — the app must belong to the caller's team (viewer).
    state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;

    let drains = state.db.list_log_drains_for_app(&app_id).await?;
    Ok(Json(serde_json::json!({ "data": drains })))
}

async fn list_global_drains(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let drains = state.db.list_global_log_drains().await?;
    Ok(Json(serde_json::json!({ "data": drains })))
}

#[derive(Deserialize)]
struct CreateDrainRequest {
    name: String,
    drain_type: String,
    config: serde_json::Value,
    #[allow(dead_code)]
    enabled: Option<bool>,
}

async fn create_drain(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(app_id): Path<String>,
    Json(body): Json<CreateDrainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let new_drain = NewLogDrain {
        app_id: Some(app_id),
        name: body.name,
        drain_type: body.drain_type,
        config: body.config.to_string(),
    };

    let drain = state.db.create_log_drain(&new_drain).await?;
    Ok(Json(serde_json::json!({ "data": drain })))
}

/// Verify a log drain bound to an app belongs to the caller's team. Drains with no
/// `app_id` are global and stay accessible to any authenticated team member.
async fn verify_drain_team_access(
    state: &AppState,
    ctx: &TeamCtx,
    drain_app_id: Option<&str>,
) -> Result<(), ApiError> {
    if let Some(app_id) = drain_app_id {
        let app = state
            .db
            .get_app_for_team(&ctx.team_id, app_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("log drain not found".into()))?;
        ctx.verify_team_access(&app.team_id, TeamRole::Member)?;
    }
    Ok(())
}

async fn update_drain(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(drain_id): Path<String>,
    Json(body): Json<CreateDrainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let existing = state
        .db
        .get_log_drain(&drain_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("log drain {drain_id} not found")))?;

    // Drain's parent app (if any) must belong to the caller's team.
    verify_drain_team_access(&state, &ctx, existing.app_id.as_deref()).await?;

    let update = NewLogDrain {
        app_id: existing.app_id,
        name: body.name,
        drain_type: body.drain_type,
        config: body.config.to_string(),
    };

    let drain = state.db.update_log_drain(&drain_id, &update).await?;
    Ok(Json(serde_json::json!({ "data": drain })))
}

async fn delete_drain(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(drain_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Drain's parent app (if any) must belong to the caller's team.
    let existing = state
        .db
        .get_log_drain(&drain_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("log drain {drain_id} not found")))?;
    verify_drain_team_access(&state, &ctx, existing.app_id.as_deref()).await?;

    state.db.delete_log_drain(&drain_id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn test_drain(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(drain_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let drain = state
        .db
        .get_log_drain(&drain_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("log drain {drain_id} not found")))?;

    // Drain's parent app (if any) must belong to the caller's team.
    verify_drain_team_access(&state, &ctx, drain.app_id.as_deref()).await?;

    let config: serde_json::Value = serde_json::from_str(&drain.config).unwrap_or_default();

    match drain.drain_type.as_str() {
        "http" => {
            let url = config["url"].as_str().unwrap_or_default();
            let method = config["method"].as_str().unwrap_or("POST");
            if url.is_empty() {
                return Err(ApiError::BadRequest(
                    "HTTP drain has no URL configured".into(),
                ));
            }
            // Block SSRF — the drain URL is user-supplied. The guarded client is pinned to the validated IP and refuses redirects.
            let target = crate::api::utils::url_guard::validate_outbound_url(
                url,
                &state.config.caddy_admin_url,
            )
            .await?;
            let client = crate::api::utils::url_guard::guarded_client(&target)?;
            let test_payload = serde_json::json!({
                "message": "Test log from Icefall",
                "level": "info",
                "timestamp": crate::db::models::now_iso8601(),
                "source": "icefall-test"
            });
            let resp = match method {
                "PUT" => {
                    client
                        .put(target.url.clone())
                        .json(&test_payload)
                        .send()
                        .await
                }
                _ => {
                    client
                        .post(target.url.clone())
                        .json(&test_payload)
                        .send()
                        .await
                }
            };
            match resp {
                Ok(r) if r.status().is_success() => Ok(Json(serde_json::json!({
                    "data": { "success": true, "message": "Test log sent successfully" }
                }))),
                Ok(r) => Err(ApiError::BadRequest(format!(
                    "Drain responded with status {}",
                    r.status()
                ))),
                Err(e) => Err(ApiError::BadRequest(format!("Failed to reach drain: {e}"))),
            }
        }
        "loki" => {
            let url = config["url"].as_str().unwrap_or_default();
            if url.is_empty() {
                return Err(ApiError::BadRequest(
                    "Loki drain has no URL configured".into(),
                ));
            }
            // Block SSRF — the drain URL is user-supplied. The guarded client is pinned to the validated IP and refuses redirects.
            let target = crate::api::utils::url_guard::validate_outbound_url(
                url,
                &state.config.caddy_admin_url,
            )
            .await?;
            let push_url = format!("{}/loki/api/v1/push", url.trim_end_matches('/'));
            let payload = serde_json::json!({
                "streams": [{
                    "stream": { "app": "icefall-test", "level": "info" },
                    "values": [[
                        format!("{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                        "Test log from Icefall"
                    ]]
                }]
            });
            let client = crate::api::utils::url_guard::guarded_client(&target)?;
            let mut req = client.post(&push_url).json(&payload);
            if let Some(tenant) = config["tenant_id"].as_str() {
                req = req.header("X-Scope-OrgID", tenant);
            }
            if let (Some(user), Some(pass)) =
                (config["username"].as_str(), config["password"].as_str())
            {
                req = req.basic_auth(user, Some(pass));
            }
            match req.send().await {
                Ok(r) if r.status().is_success() || r.status().as_u16() == 204 => {
                    Ok(Json(serde_json::json!({
                        "data": { "success": true, "message": "Test log sent to Loki" }
                    })))
                }
                Ok(r) => {
                    let body = r.text().await.unwrap_or_default();
                    Err(ApiError::BadRequest(format!(
                        "Loki responded with error: {body}"
                    )))
                }
                Err(e) => Err(ApiError::BadRequest(format!("Failed to reach Loki: {e}"))),
            }
        }
        "axiom" => {
            let dataset = config["dataset"].as_str().unwrap_or_default();
            let token = config["api_token"].as_str().unwrap_or_default();
            if dataset.is_empty() || token.is_empty() {
                return Err(ApiError::BadRequest(
                    "Axiom drain needs dataset and api_token".into(),
                ));
            }
            let url = format!("https://api.axiom.co/v1/datasets/{dataset}/ingest");
            let payload = serde_json::json!([{
                "_time": crate::db::models::now_iso8601(),
                "message": "Test log from Icefall",
                "level": "info",
            }]);
            let client = reqwest::Client::new();
            match client
                .post(&url)
                .bearer_auth(token)
                .json(&payload)
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => Ok(Json(serde_json::json!({
                    "data": { "success": true, "message": "Test log sent to Axiom" }
                }))),
                Ok(r) => {
                    let body = r.text().await.unwrap_or_default();
                    Err(ApiError::BadRequest(format!("Axiom error: {body}")))
                }
                Err(e) => Err(ApiError::BadRequest(format!("Failed to reach Axiom: {e}"))),
            }
        }
        other => Err(ApiError::BadRequest(format!("Unknown drain type: {other}"))),
    }
}
