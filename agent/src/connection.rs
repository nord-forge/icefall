use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use icefall_common::protocol::AgentMessage;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

use crate::config::AgentConfig;
use crate::context::HandlerContext;
use crate::handlers;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BACKOFF: Duration = Duration::from_secs(300);

pub async fn run_connection_loop(
    ctx: HandlerContext,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut backoff = Duration::from_secs(1);

    loop {
        if *shutdown.borrow() {
            info!("Shutdown signal received, exiting connection loop");
            break;
        }

        match connect(&ctx.config).await {
            Ok(ws) => {
                info!("Connected to control plane");
                backoff = Duration::from_secs(1);
                run_session(ws, ctx.clone(), &mut shutdown).await;

                if *shutdown.borrow() {
                    break;
                }
                warn!("Connection lost, reconnecting...");
            }
            Err(e) => {
                if e.contains("401") || e.contains("Unauthorized") {
                    error!("Authentication failed: {e}. Check your token. Exiting.");
                    std::process::exit(1);
                }
                warn!("Connection failed: {e}");
            }
        }

        let jitter = jittered_delay(backoff);
        info!("Reconnecting in {:.1}s", jitter.as_secs_f64());

        tokio::select! {
            _ = tokio::time::sleep(jitter) => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
        }

        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn connect(
    config: &AgentConfig,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    let url = config.ws_url();
    let mut request = url.into_client_request().map_err(|e| e.to_string())?;
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", config.token)).map_err(|e| e.to_string())?,
    );

    let (ws, _response) = connect_async(request).await.map_err(|e| e.to_string())?;

    Ok(ws)
}

async fn run_session(
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    mut ctx: HandlerContext,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
) {
    let (mut ws_sink, mut ws_stream) = ws.split();

    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<AgentMessage>();
    ctx.event_tx = outbound_tx.clone();

    let send_handle = tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sink.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
        let _ = ws_sink.send(Message::Close(None)).await;
    });

    let mut heartbeat_interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    let mut waiting_pong = false;
    let mut pong_deadline: Option<tokio::time::Instant> = None;

    loop {
        tokio::select! {
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_incoming(&text, &ctx, &outbound_tx);
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        handle_binary_chunk(&bytes, &ctx, &outbound_tx);
                    }
                    Some(Ok(Message::Pong(_))) => {
                        waiting_pong = false;
                        pong_deadline = None;
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Server sent close frame");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {e}");
                        break;
                    }
                    None => {
                        break;
                    }
                    _ => {}
                }
            }

            _ = heartbeat_interval.tick() => {
                if waiting_pong {
                    if let Some(deadline) = pong_deadline {
                        if tokio::time::Instant::now() >= deadline {
                            warn!("Pong timeout, closing connection");
                            break;
                        }
                    }
                } else {
                    let ping = AgentMessage::Event {
                        event_type: "ping".to_string(),
                        data: serde_json::json!({ "timestamp": chrono::Utc::now().to_rfc3339() }),
                    };
                    let _ = outbound_tx.send(ping);
                    waiting_pong = true;
                    pong_deadline = Some(tokio::time::Instant::now() + PONG_TIMEOUT);
                }
            }

            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Shutdown signal, closing WebSocket");
                    break;
                }
            }
        }
    }

    drop(outbound_tx);
    let _ = send_handle.await;
}

fn handle_incoming(
    text: &str,
    ctx: &HandlerContext,
    outbound_tx: &mpsc::UnboundedSender<AgentMessage>,
) {
    match serde_json::from_str::<AgentMessage>(text) {
        Ok(AgentMessage::Request { id, method, params }) => {
            debug!("Received request: {method} (id={id})");
            let ctx = ctx.clone();
            let tx = outbound_tx.clone();
            tokio::spawn(async move {
                let response = handlers::dispatch(&ctx, id, &method, params).await;
                let _ = tx.send(response);
            });
        }
        Ok(AgentMessage::Response { .. }) => {}
        Ok(AgentMessage::Event { event_type, .. }) => {
            debug!("Received event from CP: {event_type}");
        }
        Err(e) => {
            warn!("Failed to parse message: {e}");
        }
    }
}

/// Handle an inbound binary WebSocket frame: parse it as an image-transfer
/// chunk, verify and store it, and emit a chunk ack/nack event so the control
/// plane can retry a single failed chunk.
fn handle_binary_chunk(
    bytes: &[u8],
    ctx: &HandlerContext,
    outbound_tx: &mpsc::UnboundedSender<AgentMessage>,
) {
    use icefall_common::transfer::{hex_encode, ChunkFrame};

    let Some(frame) = ChunkFrame::decode(bytes) else {
        warn!("Received binary frame that is not a valid chunk frame");
        return;
    };

    let ctx = ctx.clone();
    let tx = outbound_tx.clone();
    tokio::spawn(async move {
        let transfer_hex = hex_encode(&frame.transfer_id);
        let (ok, error) = match ctx.transfers.accept_chunk(&frame).await {
            Ok(true) => (true, None),
            Ok(false) => (false, Some("chunk checksum mismatch".to_string())),
            Err(e) => (false, Some(e)),
        };
        let _ = tx.send(AgentMessage::Event {
            event_type: "image.load.chunk_ack".to_string(),
            data: serde_json::json!({
                "transfer_id": transfer_hex,
                "chunk_index": frame.chunk_index,
                "ok": ok,
                "error": error,
            }),
        });
    });
}

fn jittered_delay(base: Duration) -> Duration {
    use rand::Rng;
    let ms = base.as_millis() as f64;
    let jitter = rand::rng().random_range(0.8..1.2);
    Duration::from_millis((ms * jitter) as u64)
}
