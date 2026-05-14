use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::CONTROL_PLANE_SERVER_ID;

#[derive(Deserialize)]
pub struct ContainerQuery {
    #[serde(default)]
    unmanaged: bool,
}

pub(super) async fn list_server_containers(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
    Query(query): Query<ContainerQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _server = state
        .db
        .get_server(&server_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("server {server_id}")))?;

    let is_local = server_id == CONTROL_PLANE_SERVER_ID;

    let all_containers = if is_local {
        let containers = state
            .docker
            .list_containers(None)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        containers
            .into_iter()
            .map(|c| {
                let is_managed = c.labels.contains_key("icefall.app");
                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "image": c.image,
                    "status": c.status,
                    "managed": is_managed,
                    "labels": c.labels,
                })
            })
            .collect::<Vec<_>>()
    } else {
        let result = state
            .agent_registry
            .send_request(&server_id, "container.list".into(), serde_json::json!({}))
            .await
            .map_err(ApiError::BadRequest)?;

        match result {
            icefall_common::protocol::AgentMessage::Response {
                result: Some(val), ..
            } => val["containers"].as_array().cloned().unwrap_or_default(),
            _ => Vec::new(),
        }
    };

    let filtered: Vec<_> = if query.unmanaged {
        all_containers
            .into_iter()
            .filter(|c| !c.get("managed").and_then(|v| v.as_bool()).unwrap_or(false))
            .collect()
    } else {
        all_containers
    };

    Ok(Json(serde_json::json!({ "data": filtered })))
}
