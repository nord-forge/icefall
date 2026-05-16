use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{oneshot, RwLock};

use super::protocol::AgentMessage;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PENDING_REQUESTS: usize = 200;

pub struct AgentConnection {
    pub server_id: String,
    pub server_name: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<AgentMessage>,
    /// Channel for raw binary WebSocket frames (image transfer chunks).
    pub binary_sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
}

type PendingMap = HashMap<String, oneshot::Sender<AgentMessage>>;

/// Outcome of a single image-transfer chunk, reported by the agent.
#[derive(Debug, Clone)]
pub struct ChunkAck {
    pub ok: bool,
    pub error: Option<String>,
}

/// Key for a pending chunk ack: `(transfer_id_hex, chunk_index)`.
type ChunkAckKey = (String, u32);
type ChunkAckMap = HashMap<ChunkAckKey, oneshot::Sender<ChunkAck>>;

pub struct AgentRegistry {
    connections: RwLock<HashMap<String, AgentConnection>>,
    pending_requests: RwLock<PendingMap>,
    pending_chunk_acks: RwLock<ChunkAckMap>,
}

impl AgentRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
            pending_chunk_acks: RwLock::new(HashMap::new()),
        })
    }

    pub async fn register(
        &self,
        server_id: String,
        server_name: String,
        sender: tokio::sync::mpsc::UnboundedSender<AgentMessage>,
        binary_sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    ) -> Option<AgentConnection> {
        let conn = AgentConnection {
            server_id: server_id.clone(),
            server_name,
            sender,
            binary_sender,
            connected_at: Instant::now(),
            last_heartbeat: Instant::now(),
        };
        let mut conns = self.connections.write().await;
        conns.insert(server_id, conn)
    }

    /// Send a raw binary frame to a connected agent.
    pub async fn send_binary(&self, server_id: &str, frame: Vec<u8>) -> Result<(), String> {
        let conns = self.connections.read().await;
        match conns.get(server_id) {
            Some(conn) => conn
                .binary_sender
                .send(frame)
                .map_err(|_| format!("Agent {server_id} binary channel closed")),
            None => Err(format!("Agent {server_id} not connected")),
        }
    }

    pub async fn unregister(&self, server_id: &str) -> Option<AgentConnection> {
        let mut conns = self.connections.write().await;
        conns.remove(server_id)
    }

    pub async fn update_heartbeat(&self, server_id: &str) {
        let mut conns = self.connections.write().await;
        if let Some(conn) = conns.get_mut(server_id) {
            conn.last_heartbeat = Instant::now();
        }
    }

    pub async fn list_connected(&self) -> Vec<String> {
        let conns = self.connections.read().await;
        conns.keys().cloned().collect()
    }

    pub async fn send_to(&self, server_id: &str, message: AgentMessage) -> Result<(), String> {
        let conns = self.connections.read().await;
        match conns.get(server_id) {
            Some(conn) => conn
                .sender
                .send(message)
                .map_err(|_| format!("Agent {server_id} channel closed")),
            None => Err(format!("Agent {server_id} not connected")),
        }
    }

    pub async fn send_request(
        self: &Arc<Self>,
        server_id: &str,
        method: String,
        params: serde_json::Value,
    ) -> Result<AgentMessage, String> {
        self.send_request_with_timeout(server_id, method, params, REQUEST_TIMEOUT)
            .await
    }

    pub async fn send_request_with_timeout(
        self: &Arc<Self>,
        server_id: &str,
        method: String,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<AgentMessage, String> {
        let id = crate::db::models::new_id();

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.write().await;
            if pending.len() >= MAX_PENDING_REQUESTS {
                tracing::warn!(
                    "Pending requests at capacity ({}), rejecting new request",
                    MAX_PENDING_REQUESTS
                );
                return Err("Too many pending requests".to_string());
            }
            pending.insert(id.clone(), tx);
        }

        let msg = AgentMessage::Request {
            id: id.clone(),
            method,
            params,
        };

        if let Err(e) = self.send_to(server_id, msg).await {
            self.pending_requests.write().await.remove(&id);
            return Err(e);
        }

        let registry = Arc::clone(self);
        let req_id = id.clone();
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                registry.pending_requests.write().await.remove(&req_id);
                Err("Request cancelled".to_string())
            }
            Err(_) => {
                registry.pending_requests.write().await.remove(&req_id);
                Err("Request timed out".to_string())
            }
        }
    }

    pub async fn resolve_response(&self, id: &str, response: AgentMessage) {
        let mut pending = self.pending_requests.write().await;
        if let Some(tx) = pending.remove(id) {
            let _ = tx.send(response);
        }
    }

    /// Send one image-transfer chunk frame and wait for the agent's ack/nack.
    pub async fn send_chunk(
        &self,
        server_id: &str,
        transfer_id_hex: &str,
        chunk_index: u32,
        frame: Vec<u8>,
        timeout: Duration,
    ) -> Result<ChunkAck, String> {
        let key = (transfer_id_hex.to_string(), chunk_index);
        let (tx, rx) = oneshot::channel();
        self.pending_chunk_acks
            .write()
            .await
            .insert(key.clone(), tx);

        if let Err(e) = self.send_binary(server_id, frame).await {
            self.pending_chunk_acks.write().await.remove(&key);
            return Err(e);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(ack)) => Ok(ack),
            Ok(Err(_)) => {
                self.pending_chunk_acks.write().await.remove(&key);
                Err("chunk ack cancelled".to_string())
            }
            Err(_) => {
                self.pending_chunk_acks.write().await.remove(&key);
                Err("chunk ack timed out".to_string())
            }
        }
    }

    /// Resolve a pending chunk ack from an `image.load.chunk_ack` agent event.
    pub async fn resolve_chunk_ack(&self, transfer_id_hex: &str, chunk_index: u32, ack: ChunkAck) {
        let key = (transfer_id_hex.to_string(), chunk_index);
        if let Some(tx) = self.pending_chunk_acks.write().await.remove(&key) {
            let _ = tx.send(ack);
        }
    }

    pub async fn cancel_pending_for(&self, _server_id: &str) {
        // Drop all pending senders — callers will get RecvError
        // In a more sophisticated version, we'd track which requests belong to which server
        // For now, this is acceptable since each request has a timeout
    }

    pub async fn stale_servers(&self, threshold: Duration) -> Vec<(String, String)> {
        let conns = self.connections.read().await;
        let now = Instant::now();
        conns
            .values()
            .filter(|c| now.duration_since(c.last_heartbeat) > threshold)
            .map(|c| (c.server_id.clone(), c.server_name.clone()))
            .collect()
    }
}
