use std::collections::VecDeque;
use std::time::Duration;

use bollard::container::LogOutput;
use bollard::query_parameters::LogsOptions;
use futures_util::StreamExt;
use icefall_common::protocol::AgentMessage;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info, warn};

use super::HandlerError;
use crate::context::HandlerContext;

const BATCH_SIZE: usize = 50;
const BATCH_INTERVAL: Duration = Duration::from_millis(100);
const RING_BUFFER_SIZE: usize = 1000;

#[derive(Debug, Deserialize)]
struct SubscribeParams {
    container_id: String,
    #[serde(default = "default_tail")]
    tail: u64,
}

fn default_tail() -> u64 {
    50
}

#[derive(Debug, Deserialize)]
struct UnsubscribeParams {
    container_id: String,
}

pub async fn subscribe(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: SubscribeParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let mut active = ctx.active_logs.lock().await;
    if active.contains_key(&p.container_id) {
        return Ok(serde_json::json!({ "ok": true, "already_subscribed": true }));
    }

    let container_id = p.container_id.clone();
    let event_tx = ctx.event_tx.clone();
    let docker = ctx.docker.clone();
    let mut shutdown = ctx.shutdown.clone();

    let handle = tokio::spawn(async move {
        let options = LogsOptions {
            follow: true,
            stdout: true,
            stderr: true,
            tail: p.tail.to_string(),
            ..Default::default()
        };

        let mut stream = docker.logs(&container_id, Some(options));
        let mut batch: Vec<String> = Vec::with_capacity(BATCH_SIZE);
        let mut ring_buffer: VecDeque<String> = VecDeque::with_capacity(RING_BUFFER_SIZE);
        let mut flush_interval = tokio::time::interval(BATCH_INTERVAL);

        loop {
            tokio::select! {
                item = stream.next() => {
                    match item {
                        Some(Ok(output)) => {
                            let line = match output {
                                LogOutput::StdOut { message } => String::from_utf8_lossy(&message).to_string(),
                                LogOutput::StdErr { message } => String::from_utf8_lossy(&message).to_string(),
                                LogOutput::Console { message } => String::from_utf8_lossy(&message).to_string(),
                                LogOutput::StdIn { message } => String::from_utf8_lossy(&message).to_string(),
                            };

                            let trimmed = line.trim_end().to_string();
                            if trimmed.is_empty() {
                                continue;
                            }

                            if ring_buffer.len() >= RING_BUFFER_SIZE {
                                ring_buffer.pop_front();
                            }
                            ring_buffer.push_back(trimmed.clone());
                            batch.push(trimmed);

                            if batch.len() >= BATCH_SIZE {
                                flush_batch(&event_tx, &container_id, &mut batch);
                            }
                        }
                        Some(Err(e)) => {
                            warn!(container = %container_id, error = %e, "log stream error");
                            break;
                        }
                        None => {
                            debug!(container = %container_id, "log stream ended");
                            break;
                        }
                    }
                }

                _ = flush_interval.tick() => {
                    if !batch.is_empty() {
                        flush_batch(&event_tx, &container_id, &mut batch);
                    }
                }

                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        break;
                    }
                }
            }
        }

        if !batch.is_empty() {
            flush_batch(&event_tx, &container_id, &mut batch);
        }
    });

    active.insert(p.container_id.clone(), handle);
    info!(container = %p.container_id, "log streaming started");
    Ok(serde_json::json!({ "ok": true, "container_id": p.container_id }))
}

pub async fn unsubscribe(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: UnsubscribeParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let mut active = ctx.active_logs.lock().await;
    if let Some(handle) = active.remove(&p.container_id) {
        handle.abort();
        info!(container = %p.container_id, "log streaming stopped");
        Ok(serde_json::json!({ "ok": true }))
    } else {
        Err(HandlerError::Other(format!(
            "no active log subscription for container {}",
            p.container_id
        )))
    }
}

fn flush_batch(
    event_tx: &tokio::sync::mpsc::UnboundedSender<AgentMessage>,
    container_id: &str,
    batch: &mut Vec<String>,
) {
    let lines: Vec<String> = batch.drain(..).collect();
    let _ = event_tx.send(AgentMessage::Event {
        event_type: "container.logs".to_string(),
        data: serde_json::json!({
            "container_id": container_id,
            "lines": lines,
        }),
    });
}
