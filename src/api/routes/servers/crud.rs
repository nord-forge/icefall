use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::{NewServer, ServerUpdate, CONTROL_PLANE_SERVER_ID};

use super::{generate_enrollment_token, require_admin};

#[derive(Deserialize)]
pub(super) struct CreateServerRequest {
    name: String,
    host: String,
    labels: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct UpdateServerRequest {
    name: Option<String>,
    host: Option<String>,
    labels: Option<Option<String>>,
}

#[derive(Deserialize, Default)]
pub(super) struct DeleteQuery {
    force: Option<bool>,
}

pub(super) async fn create_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Server name must not be empty".into()));
    }
    if body.host.trim().is_empty() {
        return Err(ApiError::BadRequest("Server host must not be empty".into()));
    }

    let (token, token_hash) = generate_enrollment_token();

    let server = state
        .db
        .create_server(&NewServer {
            name: body.name,
            host: body.host,
            role: "worker".to_string(),
            token_hash: Some(token_hash),
            labels: body.labels,
            resources: None,
            public_key: None,
        })
        .await?;

    Ok(Json(serde_json::json!({
        "data": server,
        "meta": { "enrollment_token": token }
    })))
}

fn compute_recommendation_score(resources_json: Option<&str>, app_count: usize) -> f64 {
    let metrics: serde_json::Value = resources_json
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));

    let cpu_pct = metrics["cpu_percent"].as_f64().unwrap_or(50.0);
    let ram_used = metrics["ram_used_bytes"].as_f64().unwrap_or(0.0);
    let ram_total = metrics["ram_total_bytes"].as_f64().unwrap_or(1.0).max(1.0);
    let disk_used = metrics["disk_used_bytes"].as_f64().unwrap_or(0.0);
    let disk_total = metrics["disk_total_bytes"].as_f64().unwrap_or(1.0).max(1.0);

    let cpu_avail = (100.0 - cpu_pct) / 100.0;
    let ram_avail = 1.0 - (ram_used / ram_total);
    let disk_avail = 1.0 - (disk_used / disk_total);
    let app_factor = 1.0 / (app_count as f64 + 1.0);

    (cpu_avail + ram_avail + disk_avail + app_factor) / 4.0
}

pub(super) async fn list_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let servers = state.db.list_servers().await?;

    let apps = state.db.list_apps().await?;

    let mut best_score: Option<(usize, f64)> = None;
    let mut server_data: Vec<serde_json::Value> = Vec::new();

    for (i, s) in servers.iter().enumerate() {
        let app_count = apps
            .iter()
            .filter(|a| a.server_id.as_deref() == Some(&s.id))
            .count();

        let score = compute_recommendation_score(s.resources.as_deref(), app_count);

        let mut val = serde_json::to_value(s).unwrap_or_default();
        if let Some(obj) = val.as_object_mut() {
            obj.insert("app_count".into(), serde_json::json!(app_count));
            obj.insert("recommendation_score".into(), serde_json::json!(score));
        }
        server_data.push(val);

        if s.status == "online" {
            match best_score {
                Some((_, best)) if score > best => best_score = Some((i, score)),
                None => best_score = Some((i, score)),
                _ => {}
            }
        }
    }

    if let Some((idx, _)) = best_score {
        if let Some(obj) = server_data[idx].as_object_mut() {
            obj.insert("recommended".into(), serde_json::json!(true));
        }
    }

    Ok(Json(serde_json::json!({ "data": server_data })))
}

pub(super) async fn get_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let server = state
        .db
        .get_server(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Server {id} not found")))?;

    let apps = state.db.list_apps().await?;
    let app_count = apps
        .iter()
        .filter(|a| a.server_id.as_deref() == Some(&id))
        .count();

    let mut val = serde_json::to_value(&server).unwrap_or_default();
    if let Some(obj) = val.as_object_mut() {
        obj.insert("app_count".into(), serde_json::json!(app_count));
    }

    Ok(Json(serde_json::json!({ "data": val })))
}

pub(super) async fn update_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let server = state
        .db
        .update_server(
            &id,
            &ServerUpdate {
                name: body.name,
                host: body.host,
                status: None,
                token_hash: None,
                agent_version: None,
                labels: body.labels,
                resources: None,
                public_key: None,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": server })))
}

pub(super) async fn delete_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    if id == CONTROL_PLANE_SERVER_ID {
        return Err(ApiError::Forbidden(
            "Cannot delete the control-plane server".into(),
        ));
    }

    let apps = state.db.list_apps().await?;
    let assigned: Vec<_> = apps
        .iter()
        .filter(|a| a.server_id.as_deref() == Some(&id))
        .collect();

    if !assigned.is_empty() && query.force != Some(true) {
        return Err(ApiError::Conflict(format!(
            "{} app(s) still assigned to this server. Use ?force=true to reassign and delete.",
            assigned.len()
        )));
    }

    if !assigned.is_empty() {
        for app in &assigned {
            let _ = state
                .db
                .update_app(
                    &app.id,
                    &crate::db::models::UpdateApp {
                        server_id: Some(Some(CONTROL_PLANE_SERVER_ID.to_string())),
                        ..Default::default()
                    },
                )
                .await;
        }
    }

    state.db.delete_server(&id).await?;

    Ok(Json(serde_json::json!({ "data": { "deleted": true } })))
}

pub(super) async fn regenerate_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    state
        .db
        .get_server(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Server {id} not found")))?;

    let (token, token_hash) = generate_enrollment_token();

    state
        .db
        .update_server(
            &id,
            &ServerUpdate {
                name: None,
                host: None,
                status: None,
                token_hash: Some(Some(token_hash)),
                agent_version: None,
                labels: None,
                resources: None,
                public_key: None,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "server_id": id },
        "meta": { "enrollment_token": token }
    })))
}
