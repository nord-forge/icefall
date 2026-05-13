use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::CONTROL_PLANE_SERVER_ID;

use super::require_admin;

pub(super) async fn update_agent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let server = state
        .db
        .get_server(&server_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Server {server_id} not found")))?;

    if server.status != "online" {
        return Err(ApiError::BadRequest(format!(
            "Server '{}' is {} — cannot update while offline",
            server.name, server.status
        )));
    }

    let update_state = state.db.get_update_state().await?;
    let latest_version = update_state
        .available_version
        .as_deref()
        .unwrap_or(env!("CARGO_PKG_VERSION"));

    if server.agent_version.as_deref() == Some(latest_version) {
        return Ok(Json(serde_json::json!({
            "data": { "status": "up_to_date", "version": latest_version }
        })));
    }

    let target = server
        .resources
        .as_deref()
        .and_then(|r| serde_json::from_str::<serde_json::Value>(r).ok())
        .and_then(|v| v.get("arch")?.as_str().map(String::from))
        .unwrap_or_else(|| "x86_64-unknown-linux-musl".to_string());

    let msg = icefall_common::protocol::AgentMessage::Request {
        id: crate::db::models::new_id(),
        method: "system.update".to_string(),
        params: serde_json::json!({
            "version": latest_version,
            "target": target,
        }),
    };

    if let Err(e) = state.agent_registry.send_to(&server_id, msg).await {
        return Err(ApiError::BadRequest(format!(
            "Failed to send update command: {e}"
        )));
    }

    Ok(Json(serde_json::json!({
        "data": { "status": "update_sent", "target_version": latest_version }
    })))
}

pub(super) async fn update_all_agents(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let servers = state.db.list_servers().await?;
    let current = env!("CARGO_PKG_VERSION");
    let update_state = state.db.get_update_state().await?;
    let latest = update_state.available_version.as_deref().unwrap_or(current);

    let mut updated = 0u32;
    let mut skipped = 0u32;

    for server in &servers {
        if server.id == CONTROL_PLANE_SERVER_ID || server.status != "online" {
            skipped += 1;
            continue;
        }
        if server.agent_version.as_deref() == Some(latest) {
            skipped += 1;
            continue;
        }

        let target = server
            .resources
            .as_deref()
            .and_then(|r| serde_json::from_str::<serde_json::Value>(r).ok())
            .and_then(|v| v.get("arch")?.as_str().map(String::from))
            .unwrap_or_else(|| "x86_64-unknown-linux-musl".to_string());

        let msg = icefall_common::protocol::AgentMessage::Request {
            id: crate::db::models::new_id(),
            method: "system.update".to_string(),
            params: serde_json::json!({
                "version": latest,
                "target": target,
            }),
        };

        if state.agent_registry.send_to(&server.id, msg).await.is_ok() {
            updated += 1;
        }
    }

    Ok(Json(serde_json::json!({
        "data": { "updated": updated, "skipped": skipped, "target_version": latest }
    })))
}
