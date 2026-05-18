use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use sha2::Digest;

use crate::agent::protocol::AgentMessage;
use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::ServerUpdate;
use crate::events::EventType;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(45);
const ENROLLMENT_TOKEN_TTL_SECS: i64 = 900; // 15 minutes

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agent/ws", get(agent_ws))
        .route("/agent/enroll", post(enroll))
}

async fn agent_ws(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::BadRequest("Missing Authorization header".into()))?
        .to_string();

    let mut hasher = sha2::Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    let server = state
        .db
        .get_server_by_token_hash(&token_hash)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Invalid agent token".into()))?;

    let server_id = server.id.clone();
    let server_name = server.name.clone();

    Ok(ws.on_upgrade(move |socket| handle_agent_connection(socket, state, server_id, server_name)))
}

async fn handle_agent_connection(
    socket: WebSocket,
    state: AppState,
    server_id: String,
    server_name: String,
) {
    let (mut ws_sink, mut ws_stream) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AgentMessage>();
    let (bin_tx, mut bin_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    // Close old connection if agent reconnects
    if let Some(old) = state
        .agent_registry
        .register(server_id.clone(), server_name.clone(), tx, bin_tx)
        .await
    {
        drop(old.sender);
    }

    // Mark server online
    let _ = state.db.update_server_status(&server_id, "online").await;
    let _ = state.db.update_server_heartbeat(&server_id).await;

    state.event_bus.emit(
        EventType::ServerConnected,
        None,
        None,
        serde_json::json!({ "server_id": &server_id, "name": &server_name }),
    );

    let sid_send = server_id.clone();
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if ws_sink.send(Message::text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        None => break,
                    }
                }
                frame = bin_rx.recv() => {
                    match frame {
                        Some(frame) => {
                            if ws_sink.send(Message::binary(frame)).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
        let _ = ws_sink.close().await;
        sid_send
    });

    let registry = Arc::clone(&state.agent_registry);
    let db = Arc::clone(&state.db);
    let event_bus = Arc::clone(&state.event_bus);
    let sid_recv = server_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(agent_msg) = serde_json::from_str::<AgentMessage>(&text) {
                        match agent_msg {
                            AgentMessage::Response { ref id, .. } => {
                                let id = id.clone();
                                registry.resolve_response(&id, agent_msg).await;
                            }
                            AgentMessage::Event {
                                ref event_type,
                                ref data,
                            } => {
                                if event_type == "image.load.chunk_ack" {
                                    resolve_chunk_ack(&registry, data).await;
                                } else {
                                    forward_agent_event(&event_bus, &sid_recv, event_type, data);
                                }
                            }
                            AgentMessage::Request { .. } => {}
                        }
                    }
                }
                Message::Ping(_) => {
                    registry.update_heartbeat(&sid_recv).await;
                    let _ = db.update_server_heartbeat(&sid_recv).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        sid_recv
    });

    // Wait for either task to finish (disconnect)
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // Cleanup
    state.agent_registry.unregister(&server_id).await;
    state.agent_registry.cancel_pending_for(&server_id).await;
    let _ = state.db.update_server_status(&server_id, "offline").await;

    state.event_bus.emit(
        EventType::ServerDisconnected,
        None,
        None,
        serde_json::json!({ "server_id": &server_id, "name": &server_name, "reason": "disconnected" }),
    );
}

async fn resolve_chunk_ack(
    registry: &crate::agent::registry::AgentRegistry,
    data: &serde_json::Value,
) {
    let Some(transfer_id) = data["transfer_id"].as_str() else {
        return;
    };
    let Some(chunk_index) = data["chunk_index"].as_u64() else {
        return;
    };
    let ack = crate::agent::registry::ChunkAck {
        ok: data["ok"].as_bool().unwrap_or(false),
        error: data["error"].as_str().map(str::to_string),
    };
    registry
        .resolve_chunk_ack(transfer_id, chunk_index as u32, ack)
        .await;
}

fn forward_agent_event(
    event_bus: &crate::events::EventBus,
    server_id: &str,
    event_type: &str,
    data: &serde_json::Value,
) {
    let etype = match event_type {
        "build.step" => EventType::BuildStepStart,
        "build.output" => EventType::BuildStepOutput,
        "build.complete" => EventType::BuildComplete,
        "build.failed" => EventType::BuildComplete,
        "metrics.system" | "metrics.container" => EventType::HealthStatus,
        "container.logs" => EventType::BuildStepOutput,
        _ => return,
    };

    let mut payload = data.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("server_id".to_string(), serde_json::json!(server_id));
    }

    let app_id = data["app_id"].as_str().or(data["deploy_id"].as_str());
    event_bus.emit(etype, app_id, None, payload);
}

#[derive(Deserialize)]
struct EnrollRequest {
    enrollment_token: String,
    public_key: String,
}

async fn enroll(
    State(state): State<AppState>,
    Json(body): Json<EnrollRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use rand::RngExt;

    let mut hasher = sha2::Sha256::new();
    hasher.update(body.enrollment_token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    let server = state
        .db
        .get_server_by_token_hash(&token_hash)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Invalid enrollment token".into()))?;

    if server.status != "enrolling" {
        return Err(ApiError::BadRequest(
            "Server already enrolled or token already used".into(),
        ));
    }

    // Check TTL: token must have been generated within 15 minutes
    if let Ok(updated) = chrono::DateTime::parse_from_rfc3339(&server.updated_at) {
        let elapsed = chrono::Utc::now().signed_duration_since(updated);
        if elapsed.num_seconds() > ENROLLMENT_TOKEN_TTL_SECS {
            return Err(ApiError::BadRequest("Enrollment token expired".into()));
        }
    }

    // Generate worker token
    let random_bytes: [u8; 32] = rand::rng().random();
    let worker_token = format!("agt_{}", URL_SAFE_NO_PAD.encode(random_bytes));

    let mut worker_hasher = sha2::Sha256::new();
    worker_hasher.update(worker_token.as_bytes());
    let worker_token_hash = hex::encode(worker_hasher.finalize());

    // Update server: store worker token hash, public key, mark as online
    state
        .db
        .update_server(
            &server.id,
            &ServerUpdate {
                name: None,
                host: None,
                status: Some("online".to_string()),
                token_hash: Some(Some(worker_token_hash)),
                agent_version: None,
                labels: None,
                resources: None,
                public_key: Some(Some(body.public_key)),
            },
        )
        .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "worker_token": worker_token,
            "server_id": server.id,
        }
    })))
}

pub fn spawn_heartbeat_checker(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        loop {
            interval.tick().await;

            let stale = state.agent_registry.stale_servers(HEARTBEAT_TIMEOUT).await;
            for (server_id, server_name) in stale {
                let _ = state.db.update_server_status(&server_id, "offline").await;
                state.agent_registry.unregister(&server_id).await;

                state.event_bus.emit(
                    EventType::ServerDisconnected,
                    None,
                    None,
                    serde_json::json!({ "server_id": &server_id, "name": &server_name, "reason": "heartbeat_timeout" }),
                );
            }
        }
    });
}
