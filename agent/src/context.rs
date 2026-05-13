use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

use icefall_common::protocol::AgentMessage;

use crate::config::AgentConfig;

pub type EventSender = tokio::sync::mpsc::UnboundedSender<AgentMessage>;

pub struct TerminalSession {
    pub stdin_tx: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
    pub reader_handle: JoinHandle<()>,
}

#[derive(Clone)]
pub struct HandlerContext {
    pub docker: bollard::Docker,
    pub caddy_url: String,
    pub http: reqwest::Client,
    pub event_tx: EventSender,
    pub config: Arc<AgentConfig>,
    pub active_logs: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub active_terminals: Arc<Mutex<HashMap<String, TerminalSession>>>,
    pub shutdown: watch::Receiver<bool>,
}
