use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions};
use futures_util::StreamExt;
use icefall_common::protocol::AgentMessage;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info, warn};

use super::HandlerError;
use crate::context::{HandlerContext, TerminalSession};

#[derive(Debug, Deserialize)]
struct OpenParams {
    container_id: String,
    #[serde(default = "default_shell")]
    shell: String,
    cols: Option<u16>,
    rows: Option<u16>,
}

fn default_shell() -> String {
    "/bin/sh".to_string()
}

#[derive(Debug, Deserialize)]
struct InputParams {
    session_id: String,
    data: String,
}

#[derive(Debug, Deserialize)]
struct ResizeParams {
    session_id: String,
    cols: u16,
    rows: u16,
}

#[derive(Debug, Deserialize)]
struct CloseParams {
    session_id: String,
}

pub async fn open(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: OpenParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let exec = ctx
        .docker
        .create_exec(
            &p.container_id,
            CreateExecOptions {
                cmd: Some(vec![p.shell.clone()]),
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(true),
                ..Default::default()
            },
        )
        .await?;

    let session_id = exec.id.clone();

    if let (Some(cols), Some(rows)) = (p.cols, p.rows) {
        let _ = ctx
            .docker
            .resize_exec(
                &session_id,
                ResizeExecOptions {
                    width: cols,
                    height: rows,
                },
            )
            .await;
    }

    let start_result = ctx
        .docker
        .start_exec(
            &session_id,
            Some(StartExecOptions {
                detach: false,
                ..Default::default()
            }),
        )
        .await?;

    let event_tx = ctx.event_tx.clone();
    let sid = session_id.clone();
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel::<bytes::Bytes>();

    let reader_handle = tokio::spawn(async move {
        if let bollard::exec::StartExecResults::Attached {
            mut output,
            mut input,
        } = start_result
        {
            let mut shutdown_stdin = false;

            loop {
                tokio::select! {
                    item = output.next() => {
                        match item {
                            Some(Ok(output)) => {
                                let bytes = match output {
                                    bollard::container::LogOutput::StdOut { message } => message,
                                    bollard::container::LogOutput::StdErr { message } => message,
                                    bollard::container::LogOutput::Console { message } => message,
                                    bollard::container::LogOutput::StdIn { message } => message,
                                };
                                let text = String::from_utf8_lossy(&bytes).to_string();
                                if !text.is_empty() {
                                    let _ = event_tx.send(AgentMessage::Event {
                                        event_type: "terminal.output".to_string(),
                                        data: serde_json::json!({
                                            "session_id": sid,
                                            "data": text,
                                        }),
                                    });
                                }
                            }
                            Some(Err(e)) => {
                                warn!(session = %sid, error = %e, "terminal output error");
                                break;
                            }
                            None => {
                                debug!(session = %sid, "terminal output stream ended");
                                break;
                            }
                        }
                    }

                    data = stdin_rx.recv(), if !shutdown_stdin => {
                        match data {
                            Some(bytes) => {
                                use tokio::io::AsyncWriteExt;
                                if input.write_all(&bytes).await.is_err() {
                                    break;
                                }
                            }
                            None => {
                                shutdown_stdin = true;
                            }
                        }
                    }
                }
            }
        }
    });

    let session = TerminalSession {
        stdin_tx,
        reader_handle,
    };

    ctx.active_terminals
        .lock()
        .await
        .insert(session_id.clone(), session);

    info!(session = %session_id, container = %p.container_id, "terminal session opened");
    Ok(serde_json::json!({ "session_id": session_id }))
}

pub async fn input(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: InputParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let active = ctx.active_terminals.lock().await;
    let session = active
        .get(&p.session_id)
        .ok_or_else(|| HandlerError::Other(format!("no terminal session: {}", p.session_id)))?;

    session
        .stdin_tx
        .send(bytes::Bytes::from(p.data))
        .map_err(|_| HandlerError::Other("terminal stdin closed".into()))?;

    Ok(serde_json::json!({ "ok": true }))
}

pub async fn resize(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: ResizeParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    ctx.docker
        .resize_exec(
            &p.session_id,
            ResizeExecOptions {
                width: p.cols,
                height: p.rows,
            },
        )
        .await?;

    Ok(serde_json::json!({ "ok": true }))
}

pub async fn close(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: CloseParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let mut active = ctx.active_terminals.lock().await;
    if let Some(session) = active.remove(&p.session_id) {
        session.reader_handle.abort();
        info!(session = %p.session_id, "terminal session closed");
        Ok(serde_json::json!({ "ok": true }))
    } else {
        Err(HandlerError::Other(format!(
            "no terminal session: {}",
            p.session_id
        )))
    }
}
