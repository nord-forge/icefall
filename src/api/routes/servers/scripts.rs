use axum::extract::{Path, State};
use axum::response::IntoResponse;

use crate::api::error::ApiError;
use crate::api::AppState;

pub(super) async fn setup_script(State(state): State<AppState>) -> impl IntoResponse {
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

pub(super) async fn download_agent(
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

pub(super) async fn uninstall_script() -> impl IntoResponse {
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
