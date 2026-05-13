use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use sha2::Digest;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::{NewServer, ServerUpdate, CONTROL_PLANE_SERVER_ID};

#[derive(Deserialize)]
struct CreateServerRequest {
    name: String,
    host: String,
    labels: Option<String>,
}

#[derive(Deserialize)]
struct UpdateServerRequest {
    name: Option<String>,
    host: Option<String>,
    labels: Option<Option<String>>,
}

#[derive(Deserialize, Default)]
struct DeleteQuery {
    force: Option<bool>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/servers", get(list_servers).post(create_server))
        .route("/servers/setup", get(setup_script))
        .route(
            "/servers/{id}",
            get(get_server).put(update_server).delete(delete_server),
        )
        .route("/servers/{id}/token", post(regenerate_token))
        .route("/servers/{id}/update", post(update_agent))
        .route("/servers/update-all", post(update_all_agents))
        .route("/agent/download/{target}", get(download_agent))
        .route("/agent/uninstall", get(uninstall_script))
}

async fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let user = authenticate_from_headers(state, headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if user.role != "admin" {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    Ok(())
}

fn generate_enrollment_token() -> (String, String) {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use rand::Rng;

    let random_bytes: [u8; 32] = rand::rng().random();
    let token = URL_SAFE_NO_PAD.encode(random_bytes);

    let mut hasher = sha2::Sha256::new();
    hasher.update(token.as_bytes());
    let hash = hex::encode(hasher.finalize());

    (token, hash)
}

async fn create_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Server name must not be empty".into()));
    }
    if body.host.trim().is_empty() {
        return Err(ApiError::BadRequest("Server host must not be empty".into()));
    }

    let (token, token_hash) = generate_enrollment_token();

    let server = state
        .db
        .create_server(&NewServer {
            name: body.name,
            host: body.host,
            role: "worker".to_string(),
            token_hash: Some(token_hash),
            labels: body.labels,
            resources: None,
            public_key: None,
        })
        .await?;

    Ok(Json(serde_json::json!({
        "data": server,
        "meta": { "enrollment_token": token }
    })))
}

fn compute_recommendation_score(resources_json: Option<&str>, app_count: usize) -> f64 {
    let metrics: serde_json::Value = resources_json
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));

    let cpu_pct = metrics["cpu_percent"].as_f64().unwrap_or(50.0);
    let ram_used = metrics["ram_used_bytes"].as_f64().unwrap_or(0.0);
    let ram_total = metrics["ram_total_bytes"].as_f64().unwrap_or(1.0).max(1.0);
    let disk_used = metrics["disk_used_bytes"].as_f64().unwrap_or(0.0);
    let disk_total = metrics["disk_total_bytes"].as_f64().unwrap_or(1.0).max(1.0);

    let cpu_avail = (100.0 - cpu_pct) / 100.0;
    let ram_avail = 1.0 - (ram_used / ram_total);
    let disk_avail = 1.0 - (disk_used / disk_total);
    let app_factor = 1.0 / (app_count as f64 + 1.0);

    (cpu_avail + ram_avail + disk_avail + app_factor) / 4.0
}

async fn list_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let servers = state.db.list_servers().await?;

    let apps = state.db.list_apps().await?;

    let mut best_score: Option<(usize, f64)> = None;
    let mut server_data: Vec<serde_json::Value> = Vec::new();

    for (i, s) in servers.iter().enumerate() {
        let app_count = apps
            .iter()
            .filter(|a| a.server_id.as_deref() == Some(&s.id))
            .count();

        let score = compute_recommendation_score(s.resources.as_deref(), app_count);

        let mut val = serde_json::to_value(s).unwrap_or_default();
        if let Some(obj) = val.as_object_mut() {
            obj.insert("app_count".into(), serde_json::json!(app_count));
            obj.insert("recommendation_score".into(), serde_json::json!(score));
        }
        server_data.push(val);

        if s.status == "online" {
            match best_score {
                Some((_, best)) if score > best => best_score = Some((i, score)),
                None => best_score = Some((i, score)),
                _ => {}
            }
        }
    }

    if let Some((idx, _)) = best_score {
        if let Some(obj) = server_data[idx].as_object_mut() {
            obj.insert("recommended".into(), serde_json::json!(true));
        }
    }

    Ok(Json(serde_json::json!({ "data": server_data })))
}

async fn get_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let server = state
        .db
        .get_server(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Server {id} not found")))?;

    let apps = state.db.list_apps().await?;
    let app_count = apps
        .iter()
        .filter(|a| a.server_id.as_deref() == Some(&id))
        .count();

    let mut val = serde_json::to_value(&server).unwrap_or_default();
    if let Some(obj) = val.as_object_mut() {
        obj.insert("app_count".into(), serde_json::json!(app_count));
    }

    Ok(Json(serde_json::json!({ "data": val })))
}

async fn update_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    let server = state
        .db
        .update_server(
            &id,
            &ServerUpdate {
                name: body.name,
                host: body.host,
                status: None,
                token_hash: None,
                agent_version: None,
                labels: body.labels,
                resources: None,
                public_key: None,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": server })))
}

async fn delete_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    if id == CONTROL_PLANE_SERVER_ID {
        return Err(ApiError::Forbidden(
            "Cannot delete the control-plane server".into(),
        ));
    }

    let apps = state.db.list_apps().await?;
    let assigned: Vec<_> = apps
        .iter()
        .filter(|a| a.server_id.as_deref() == Some(&id))
        .collect();

    if !assigned.is_empty() && query.force != Some(true) {
        return Err(ApiError::Conflict(format!(
            "{} app(s) still assigned to this server. Use ?force=true to reassign and delete.",
            assigned.len()
        )));
    }

    if !assigned.is_empty() {
        for app in &assigned {
            let _ = state
                .db
                .update_app(
                    &app.id,
                    &crate::db::models::UpdateApp {
                        name: None,
                        git_repo: None,
                        git_branch: None,
                        framework: None,
                        build_config: None,
                        resource_limits: None,
                        preview_enabled: None,
                        preview_branch_pattern: None,
                        tags: None,
                        volumes: None,
                        image_ref: None,
                        compose_content: None,
                        project_id: None,
                        deploy_mode: None,
                        server_id: Some(Some(CONTROL_PLANE_SERVER_ID.to_string())),
                    },
                )
                .await;
        }
    }

    state.db.delete_server(&id).await?;

    Ok(Json(serde_json::json!({ "data": { "deleted": true } })))
}

async fn regenerate_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;

    state
        .db
        .get_server(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Server {id} not found")))?;

    let (token, token_hash) = generate_enrollment_token();

    state
        .db
        .update_server(
            &id,
            &ServerUpdate {
                name: None,
                host: None,
                status: None,
                token_hash: Some(Some(token_hash)),
                agent_version: None,
                labels: None,
                resources: None,
                public_key: None,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "server_id": id },
        "meta": { "enrollment_token": token }
    })))
}

async fn setup_script(State(state): State<AppState>) -> impl IntoResponse {
    let base_url = match state.config.base_domain.as_deref() {
        Some(domain) => format!("https://{domain}"),
        None => format!(
            "http://{}:{}",
            state.config.listen_addr, state.config.listen_port
        ),
    };

    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

# Icefall Agent Install Script
# Usage: curl -fsSL {base_url}/api/v1/servers/setup | bash -s -- --token <enrollment-token>

ICEFALL_URL="{base_url}"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/icefall-agent"
DATA_DIR="/var/lib/icefall-agent"
TOKEN=""

# --- Color helpers ---
if [ -t 1 ] && [ -z "${{NO_COLOR:-}}" ]; then
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    GREEN='' YELLOW='' RED='' BOLD='' NC=''
fi

info()  {{ printf "${{GREEN}}[✓]${{NC}} %s\n" "$1"; }}
warn()  {{ printf "${{YELLOW}}[!]${{NC}} %s\n" "$1"; }}
fail()  {{ printf "${{RED}}[✗]${{NC}} %s\n" "$1"; exit 1; }}
step()  {{ printf "${{BOLD}}→${{NC}} %s\n" "$1"; }}

# --- Parse arguments ---
while [ $# -gt 0 ]; do
    case "$1" in
        --token) TOKEN="${{2:?--token requires a value}}"; shift 2 ;;
        --token=*) TOKEN="${{1#*=}}"; shift ;;
        *) TOKEN="$1"; shift ;;
    esac
done

[ -z "$TOKEN" ] && fail "Missing enrollment token. Usage: $0 --token <enrollment-token>"

echo ""
printf "${{BOLD}}Icefall Agent Installer${{NC}}\n"
echo "Control plane: $ICEFALL_URL"
echo ""

# --- Detect architecture ---
step "Detecting architecture..."
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)       TARGET="x86_64-linux" ;;
    aarch64|arm64) TARGET="aarch64-linux" ;;
    *) fail "Unsupported architecture: $ARCH (only x86_64 and aarch64 are supported)" ;;
esac
info "Architecture: $TARGET"

# --- Install Docker if missing ---
if command -v docker &>/dev/null; then
    info "Docker already installed ($(docker --version))"
else
    step "Installing Docker..."
    curl -fsSL https://get.docker.com | sh || fail "Docker installation failed. Install Docker manually and re-run."
    sudo systemctl enable --now docker || fail "Failed to start Docker daemon"
    sudo usermod -aG docker "$(whoami)" 2>/dev/null || true
    info "Docker installed"
fi

# Verify Docker daemon is running
sudo docker info &>/dev/null || fail "Docker daemon is not running. Start it with: sudo systemctl start docker"

# --- Install Caddy if missing ---
if command -v caddy &>/dev/null; then
    info "Caddy already installed ($(caddy version 2>/dev/null || echo 'unknown'))"
else
    step "Installing Caddy..."
    if [ -f /etc/debian_version ]; then
        sudo apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl &>/dev/null
        curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg 2>/dev/null
        curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list &>/dev/null
        sudo apt-get update &>/dev/null && sudo apt-get install -y caddy &>/dev/null
    elif [ -f /etc/redhat-release ]; then
        sudo dnf install -y 'dnf-command(copr)' &>/dev/null
        sudo dnf copr enable -y @caddy/caddy &>/dev/null
        sudo dnf install -y caddy &>/dev/null
    else
        fail "Unsupported OS for automatic Caddy install. Install Caddy manually: https://caddyserver.com/docs/install"
    fi
    sudo systemctl enable --now caddy || fail "Failed to start Caddy"
    info "Caddy installed"
fi

# --- Download agent binary ---
step "Downloading agent binary..."
DOWNLOAD_URL="$ICEFALL_URL/api/v1/agent/download/$TARGET"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

curl -fsSL -o "$TMP_DIR/icefall-agent" "$DOWNLOAD_URL" || fail "Failed to download agent binary from $DOWNLOAD_URL"
curl -fsSL -o "$TMP_DIR/icefall-agent.sha256" "$DOWNLOAD_URL.sha256" 2>/dev/null && {{
    step "Verifying checksum..."
    EXPECTED=$(cat "$TMP_DIR/icefall-agent.sha256" | awk '{{print $1}}')
    ACTUAL=$(sha256sum "$TMP_DIR/icefall-agent" | awk '{{print $1}}')
    [ "$EXPECTED" = "$ACTUAL" ] || fail "Checksum mismatch! Expected $EXPECTED, got $ACTUAL"
    info "Checksum verified"
}} || warn "Checksum file not available, skipping verification"

sudo mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$DATA_DIR"
sudo cp "$TMP_DIR/icefall-agent" "$INSTALL_DIR/icefall-agent"
sudo chmod +x "$INSTALL_DIR/icefall-agent"
info "Binary installed to $INSTALL_DIR/icefall-agent"

# --- Enroll with control plane ---
step "Enrolling with control plane..."
sudo "$INSTALL_DIR/icefall-agent" enroll \
    --url "$ICEFALL_URL" \
    --token "$TOKEN" \
    --data-dir "$DATA_DIR" || fail "Enrollment failed. Check that the token is valid and not expired."
info "Enrollment complete"

# --- Install systemd service ---
step "Installing systemd service..."
sudo tee /etc/systemd/system/icefall-agent.service > /dev/null <<UNIT
[Unit]
Description=Icefall Agent
After=network-online.target docker.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/icefall-agent run --config $DATA_DIR/config.toml
Restart=always
RestartSec=5
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=$CONFIG_DIR $DATA_DIR

[Install]
WantedBy=multi-user.target
UNIT

sudo systemctl daemon-reload
sudo systemctl enable --now icefall-agent
info "Service installed and started"

echo ""
printf "${{GREEN}}${{BOLD}}Agent installed and running.${{NC}}\n"
echo "Check status: sudo systemctl status icefall-agent"
echo "View logs:    sudo journalctl -u icefall-agent -f"
echo ""
"#
    );

    ([("content-type", "text/x-shellscript")], script)
}

async fn download_agent(
    State(state): State<AppState>,
    Path(target): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let is_checksum = target.ends_with(".sha256");
    let actual_target = if is_checksum {
        target.trim_end_matches(".sha256")
    } else {
        &target
    };

    match actual_target {
        "x86_64-linux" | "aarch64-linux" => {}
        _ => {
            return Err(ApiError::NotFound(format!(
                "Unknown target: {actual_target}. Supported: x86_64-linux, aarch64-linux"
            )));
        }
    }

    let binary_name = format!("icefall-agent-{actual_target}");
    let artifacts_dir = state.config.data_dir.join("agent-releases");

    if is_checksum {
        let checksum_path = artifacts_dir.join(format!("{binary_name}.sha256"));
        let content = tokio::fs::read_to_string(&checksum_path)
            .await
            .map_err(|_| {
                ApiError::NotFound(
                    "Checksum file not found. Agent binary may not have been released yet.".into(),
                )
            })?;
        Ok(([("content-type", "text/plain")], content.into_bytes()))
    } else {
        let binary_path = artifacts_dir.join(&binary_name);
        let content = tokio::fs::read(&binary_path).await.map_err(|_| {
            ApiError::NotFound(
                "Agent binary not found. Build and place it in the agent-releases directory."
                    .into(),
            )
        })?;
        Ok(([("content-type", "application/octet-stream")], content))
    }
}

async fn update_agent(
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

async fn update_all_agents(
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

async fn uninstall_script() -> impl IntoResponse {
    let script = r#"#!/usr/bin/env bash
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root (use sudo)"
    exit 1
fi

echo "Icefall Agent Uninstall"
echo ""

# Stop and disable service
if command -v systemctl &>/dev/null && systemctl is-active --quiet icefall-agent 2>/dev/null; then
    echo "Stopping icefall-agent service..."
    systemctl stop icefall-agent
    systemctl disable icefall-agent
    rm -f /etc/systemd/system/icefall-agent.service
    systemctl daemon-reload
    echo "Service removed."
elif [ -f /etc/init.d/icefall-agent ]; then
    rc-service icefall-agent stop 2>/dev/null || true
    rc-update del icefall-agent default 2>/dev/null || true
    rm -f /etc/init.d/icefall-agent
    echo "OpenRC service removed."
fi

# Remove binary
if [ -f /usr/local/bin/icefall-agent ]; then
    rm -f /usr/local/bin/icefall-agent
    rm -f /usr/local/bin/icefall-agent.prev
    echo "Binary removed."
fi

# Remove config (with confirmation)
if [ -d /etc/icefall-agent ]; then
    if [ "${1:-}" = "--yes" ]; then
        rm -rf /etc/icefall-agent
        echo "Config directory removed."
    else
        read -rp "Remove /etc/icefall-agent config directory? [y/N] " response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            rm -rf /etc/icefall-agent
            echo "Config directory removed."
        else
            echo "Config directory kept."
        fi
    fi
fi

echo ""
echo "Icefall agent uninstalled. Docker and Caddy were not removed."
"#;
    ([("content-type", "text/x-shellscript")], script.to_string())
}
