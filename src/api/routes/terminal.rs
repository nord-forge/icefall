use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::query_parameters::ResizeExecOptions;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::api::error::ApiError;
use crate::api::routes::auth::extract_session_id;
use crate::api::AppState;

const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub fn routes() -> Router<AppState> {
    Router::new().route("/apps/{id}/terminal", get(terminal_ws))
}

#[derive(Deserialize)]
struct TerminalQuery {
    token: Option<String>,
}

async fn terminal_ws(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<TerminalQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ApiError> {
    // Authenticate via query-param token (WebSocket clients can't set
    // Authorization headers) or the session cookie. Resolve to the user id
    // so we can enforce team access — a valid token alone is not enough.
    let user_id = if let Some(ref token) = params.token {
        resolve_user(&state, token).await?
    } else if let Some(session_id) = extract_session_id(&headers) {
        resolve_user(&state, &session_id).await?
    } else {
        None
    };
    let user_id = user_id.ok_or_else(|| ApiError::Forbidden("Authentication required".into()))?;

    // H5: a root shell in a container is one of the most powerful actions in
    // the product. Verify the caller's team owns this app before allowing it
    // — previously any authenticated user could open a terminal to ANY app.
    let team_id = resolve_user_team(&state, &user_id).await?;
    state
        .db
        .get_app_for_team(&team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    // Find the running container for this app
    let label = format!("icefall.app={id}");
    let containers = state.docker.list_containers(Some(&label)).await?;
    let container = containers
        .iter()
        .find(|c| c.state == "running")
        .ok_or_else(|| ApiError::BadRequest("No running container found for this app".into()))?;

    let container_id = container.id.clone();
    let docker = state.docker.clone();

    Ok(ws.on_upgrade(move |socket| handle_terminal(socket, docker, container_id)))
}

/// Resolve a terminal auth token (API token or session id) to the owning
/// user id. Returns `None` if the token is invalid or expired.
async fn resolve_user(state: &AppState, token: &str) -> Result<Option<String>, ApiError> {
    // Try as API token first (icefall_ prefix).
    if token.starts_with("icefall_") {
        let token_hash = sha256_hex(token);
        if let Some(api_token) = state.db.get_api_token_by_hash(&token_hash).await? {
            if let Some(ref exp) = api_token.expires_at {
                if exp < &crate::db::models::now_iso8601() {
                    return Ok(None);
                }
            }
            let _ = state.db.update_token_last_used(&api_token.id).await;
            return Ok(Some(api_token.user_id));
        }
    }

    // Try as session token.
    if let Some(session) = state.db.get_session(token).await? {
        if session.expires_at >= crate::db::models::now_iso8601() {
            return Ok(Some(session.user_id));
        }
        state.db.delete_session(token).await?;
    }

    Ok(None)
}

/// Resolve the team a user acts under for terminal access — their first
/// (personal) team. Under the always-a-team model every user owns one.
async fn resolve_user_team(state: &AppState, user_id: &str) -> Result<String, ApiError> {
    state
        .db
        .list_teams_for_user(user_id)
        .await?
        .into_iter()
        .next()
        .map(|t| t.id)
        .ok_or_else(|| ApiError::Forbidden("No team context for this user".into()))
}

fn sha256_hex(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Deserialize)]
struct ResizeMessage {
    #[serde(rename = "type")]
    msg_type: String,
    cols: u16,
    rows: u16,
}

async fn handle_terminal(
    socket: WebSocket,
    docker: std::sync::Arc<crate::docker::DockerClient>,
    container_id: String,
) {
    if let Err(e) = handle_terminal_inner(socket, docker, container_id).await {
        tracing::warn!("terminal session ended with error: {e}");
    }
}

async fn handle_terminal_inner(
    socket: WebSocket,
    docker: std::sync::Arc<crate::docker::DockerClient>,
    container_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut ws_sink, mut ws_stream) = socket.split();

    // Create a docker exec with TTY
    let exec = docker
        .inner()
        .create_exec(
            &container_id,
            CreateExecOptions {
                cmd: Some(vec!["/bin/sh".to_string()]),
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(true),
                ..Default::default()
            },
        )
        .await?;

    let exec_id = exec.id.clone();

    // Start the exec and get the bidirectional stream
    let start_result = docker
        .inner()
        .start_exec(
            &exec.id,
            Some(StartExecOptions {
                detach: false,
                tty: true,
                ..Default::default()
            }),
        )
        .await?;

    let (mut docker_output, mut docker_input) = match start_result {
        StartExecResults::Attached { output, input } => (output, input),
        StartExecResults::Detached => {
            let _ = ws_sink.send(Message::Close(None)).await;
            return Ok(());
        }
    };

    // Spawn a task to read from Docker and send to WebSocket
    let docker_to_ws = tokio::spawn(async move {
        while let Some(result) = docker_output.next().await {
            match result {
                Ok(output) => {
                    let bytes = output.into_bytes();
                    if ws_sink.send(Message::Binary(bytes)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::debug!("docker exec output error: {e}");
                    break;
                }
            }
        }
        let _ = ws_sink.send(Message::Close(None)).await;
    });

    // Read from WebSocket and write to Docker stdin, with inactivity timeout
    let ws_to_docker = async {
        loop {
            let msg = tokio::time::timeout(INACTIVITY_TIMEOUT, ws_stream.next()).await;

            match msg {
                #[allow(clippy::collapsible_match)]
                Ok(Some(Ok(message))) => match message {
                    Message::Text(text) => {
                        // Check for resize messages
                        if let Ok(resize) = serde_json::from_str::<ResizeMessage>(&text) {
                            if resize.msg_type == "resize" {
                                let _ = docker
                                    .inner()
                                    .resize_exec(
                                        &exec_id,
                                        ResizeExecOptions {
                                            w: resize.cols as i32,
                                            h: resize.rows as i32,
                                        },
                                    )
                                    .await;
                                continue;
                            }
                        }
                        // Regular text input
                        if docker_input.write_all(text.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    Message::Binary(data) => {
                        if docker_input.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                },
                Ok(Some(Err(_))) | Ok(None) => break,
                Err(_) => {
                    // Inactivity timeout
                    tracing::info!("terminal session timed out due to inactivity");
                    break;
                }
            }
        }
    };

    ws_to_docker.await;

    // Abort the docker-to-ws task when the ws-to-docker loop exits
    docker_to_ws.abort();

    Ok(())
}
