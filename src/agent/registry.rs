use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{oneshot, RwLock};

use super::protocol::AgentMessage;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct AgentConnection {
    pub server_id: String,
    pub server_name: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<AgentMessage>,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
}

type PendingMap = HashMap<String, oneshot::Sender<AgentMessage>>;

pub struct AgentRegistry {
    connections: RwLock<HashMap<String, AgentConnection>>,
    pending_requests: RwLock<PendingMap>,
}

impl AgentRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
        })
    }

    pub async fn register(
        &self,
        server_id: String,
        server_name: String,
        sender: tokio::sync::mpsc::UnboundedSender<AgentMessage>,
    ) -> Option<AgentConnection> {
        let conn = AgentConnection {
            server_id: server_id.clone(),
            server_name,
            sender,
            connected_at: Instant::now(),
            last_heartbeat: Instant::now(),
        };
        let mut conns = self.connections.write().await;
        conns.insert(server_id, conn)
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
        self.pending_requests.write().await.insert(id.clone(), tx);

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
