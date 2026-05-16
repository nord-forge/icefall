#!/usr/bin/env bash
# Icefall Installation Script
# Usage: curl -fsSL https://icefall.dev/install.sh | bash
#
# Environment variables:
#   ICEFALL_VERSION   - Version to install (default: latest)
#   ICEFALL_RUNTIME   - Container runtime: docker | podman | auto (default: auto)
#   NO_COLOR          - Disable colored output
#
# Flags (positional, any order):
#   --yes             - Non-interactive; accept defaults
#   --runtime=NAME    - Container runtime: docker | podman | auto

set -euo pipefail

ICEFALL_VERSION="${ICEFALL_VERSION:-latest}"
ICEFALL_BIN="/usr/local/bin/icefall"
ICEFALL_DATA="/var/lib/icefall"
ICEFALL_CONFIG="/etc/icefall/config.toml"
ICEFALL_SERVICE="/etc/systemd/system/icefall.service"
ICEFALL_LOG="/var/log/icefall-install.log"

# Runtime preference: docker | podman | auto. Env var is the default; a
# --runtime= flag overrides it. "auto" (or empty) means detect/prompt.
RUNTIME_CHOICE="${ICEFALL_RUNTIME:-auto}"
NONINTERACTIVE=""

for _arg in "$@"; do
    case "$_arg" in
        --yes)
            NONINTERACTIVE="--yes"
            ;;
        --runtime=*)
            RUNTIME_CHOICE="${_arg#--runtime=}"
            ;;
    esac
done

case "$RUNTIME_CHOICE" in
    docker|podman|auto) ;;
    *) echo "Invalid --runtime / ICEFALL_RUNTIME value: '$RUNTIME_CHOICE' (expected docker, podman, or auto)" >&2; exit 1 ;;
esac

if [ -n "${NO_COLOR:-}" ]; then
    BLUE="" GREEN="" YELLOW="" RED="" BOLD="" RESET=""
else
    BLUE="\033[1;34m" GREEN="\033[1;32m" YELLOW="\033[1;33m" RED="\033[1;31m" BOLD="\033[1m" RESET="\033[0m"
fi

info()  { echo -e "${BLUE}[icefall]${RESET} $*" | tee -a "$ICEFALL_LOG"; }
warn()  { echo -e "${YELLOW}[warn]${RESET} $*" | tee -a "$ICEFALL_LOG"; }
error() { echo -e "${RED}[error]${RESET} $*" | tee -a "$ICEFALL_LOG"; exit 1; }
ok()    { echo -e "${GREEN}[ok]${RESET} $*" | tee -a "$ICEFALL_LOG"; }

trap 'error "Install failed at line $LINENO (command: $BASH_COMMAND)"' ERR

confirm() {
    if [ "$NONINTERACTIVE" = "--yes" ]; then return 0; fi
    read -rp "$1 [y/N] " response
    [[ "$response" =~ ^[Yy]$ ]]
}

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS_ID="${ID:-unknown}"
        OS_VERSION="${VERSION_ID:-0}"
    else
        error "Cannot detect OS. /etc/os-release not found. Supported: Ubuntu 20.04+, Debian 11+, CentOS/Rocky/Alma 8+, Fedora 38+, Alpine 3.16+"
    fi

    case "$OS_ID" in
        ubuntu|debian|centos|rhel|rocky|almalinux|fedora|alpine)
            ok "Detected $OS_ID $OS_VERSION"
            ;;
        *)
            warn "Unsupported OS: $OS_ID $OS_VERSION. Proceeding anyway — manual intervention may be needed."
            ;;
    esac
}

detect_arch() {
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64)  ARCH="x86_64" ;;
        aarch64) ARCH="aarch64" ;;
        arm64)   ARCH="aarch64" ;;
        *) error "Unsupported architecture: $ARCH. Supported: x86_64, aarch64" ;;
    esac
}

is_alpine() { [ "$OS_ID" = "alpine" ]; }

check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        error "This script must be run as root (use: sudo bash install.sh)"
    fi
}

check_prereqs() {
    info "Checking prerequisites..."

    if ! command -v curl &>/dev/null && ! command -v wget &>/dev/null; then
        error "curl or wget is required"
    fi
    ok "curl/wget available"

    if ! is_alpine && ! command -v systemctl &>/dev/null; then
        error "systemd is required (not found). Alpine uses OpenRC."
    fi

    install_container_runtime
}

CONTAINER_RUNTIME=""
CONTAINER_SOCKET=""

detect_container_runtime() {
    if command -v docker &>/dev/null && docker info &>/dev/null 2>&1; then
        CONTAINER_RUNTIME="docker"
        CONTAINER_SOCKET="/var/run/docker.sock"
        ok "Docker $(docker --version | cut -d' ' -f3 | tr -d ',') detected (running)"
        return 0
    fi

    if command -v podman &>/dev/null && podman info &>/dev/null 2>&1; then
        CONTAINER_RUNTIME="podman"
        if [ -S "/run/podman/podman.sock" ]; then
            CONTAINER_SOCKET="/run/podman/podman.sock"
        else
            CONTAINER_SOCKET="/var/run/podman/podman.sock"
        fi
        local podman_version
        podman_version=$(podman --version | awk '{print $3}')
        local podman_major
        podman_major=$(echo "$podman_version" | cut -d. -f1)
        if [ "$podman_major" -lt 4 ]; then
            warn "Podman $podman_version detected but Icefall requires >= 4.0"
            return 1
        fi
        ok "Podman $podman_version detected (running)"
        return 0
    fi

    return 1
}

ensure_podman_socket() {
    if is_alpine; then
        return 0
    fi
    if ! systemctl is-active --quiet podman.socket 2>/dev/null; then
        info "Enabling Podman API socket..."
        systemctl enable --now podman.socket 2>/dev/null || true
    fi
    ok "Podman socket active"
}

# Read the runtime from an existing config so a re-install never flips a
# server's runtime. Sets RUNTIME_CHOICE to the configured value if found.
adopt_runtime_from_config() {
    if [ ! -f "$ICEFALL_CONFIG" ]; then
        return 1
    fi
    local configured
    configured=$(grep -E '^\s*runtime\s*=' "$ICEFALL_CONFIG" 2>/dev/null \
        | head -1 | sed -E 's/.*=\s*"?([a-z]+)"?.*/\1/')
    case "$configured" in
        docker|podman)
            RUNTIME_CHOICE="$configured"
            info "Existing install uses '$configured' — keeping that runtime"
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# Ensure Docker is installed and running; install it if missing. Used by
# forced (--runtime=docker) mode — never falls back to another runtime.
ensure_docker() {
    if command -v docker &>/dev/null && docker info &>/dev/null 2>&1; then
        CONTAINER_RUNTIME="docker"
        CONTAINER_SOCKET="/var/run/docker.sock"
        ok "Docker $(docker --version | cut -d' ' -f3 | tr -d ',') detected (running)"
        return 0
    fi
    if command -v docker &>/dev/null; then
        warn "Docker is installed but not running — starting it"
        if is_alpine; then
            rc-service docker start 2>/dev/null || true
        else
            systemctl start docker 2>/dev/null || true
        fi
        if docker info &>/dev/null 2>&1; then
            CONTAINER_RUNTIME="docker"
            CONTAINER_SOCKET="/var/run/docker.sock"
            ok "Docker started"
            return 0
        fi
        error "Docker is installed but failed to start. Fix Docker, then re-run."
    fi
    install_docker
}

# Ensure Podman is installed and running; install it if missing. Used by
# forced (--runtime=podman) mode — never falls back to another runtime.
ensure_podman() {
    if command -v podman &>/dev/null; then
        ensure_podman_socket
        if podman info &>/dev/null 2>&1; then
            local podman_version podman_major
            podman_version=$(podman --version | awk '{print $3}')
            podman_major=$(echo "$podman_version" | cut -d. -f1)
            if [ "$podman_major" -lt 4 ]; then
                error "Podman $podman_version detected but Icefall requires >= 4.0. Upgrade Podman, then re-run."
            fi
            CONTAINER_RUNTIME="podman"
            if [ -S "/run/podman/podman.sock" ]; then
                CONTAINER_SOCKET="/run/podman/podman.sock"
            else
                CONTAINER_SOCKET="/var/run/podman/podman.sock"
            fi
            ok "Podman $podman_version detected (running)"
            return 0
        fi
        error "Podman is installed but its API socket is not reachable. Run 'systemctl enable --now podman.socket', then re-run."
    fi
    install_podman
}

# When both runtimes are installed and the user has not forced a choice,
# offer an explicit pick (with an auto-detect fallback) so a Podman-committed
# user is not silently given Docker.
prompt_runtime_choice() {
    # Only relevant in interactive auto mode.
    [ "$RUNTIME_CHOICE" = "auto" ] || return 0
    [ "$NONINTERACTIVE" = "--yes" ] && return 0

    local have_docker=false have_podman=false
    command -v docker &>/dev/null && have_docker=true
    command -v podman &>/dev/null && have_podman=true

    # Only prompt when there is an actual choice to make.
    if [ "$have_docker" = true ] && [ "$have_podman" = true ]; then
        info "Both Docker and Podman are installed."
        echo ""
        echo "  Which container runtime should Icefall use?"
        echo ""
        echo "    1) Docker"
        echo "    2) Podman"
        echo "    3) Auto-detect     — recommended, pick for me"
        echo ""
        local choice
        read -rp "Use [1/2/3]: " choice
        case "$choice" in
            1) RUNTIME_CHOICE="docker" ;;
            2) RUNTIME_CHOICE="podman" ;;
            *) RUNTIME_CHOICE="auto" ;;
        esac
    fi
}

install_container_runtime() {
    # A re-install always keeps the runtime the server was set up with,
    # unless the user explicitly overrides it with --runtime / ICEFALL_RUNTIME.
    if [ "$RUNTIME_CHOICE" = "auto" ]; then
        adopt_runtime_from_config || true
    fi

    # If still on auto and both runtimes exist, let the user choose.
    if [ "$RUNTIME_CHOICE" = "auto" ]; then
        prompt_runtime_choice
    fi

    # Forced runtime — honor it exactly, never fall back to the other.
    case "$RUNTIME_CHOICE" in
        docker)
            info "Using Docker (requested explicitly)"
            ensure_docker
            return
            ;;
        podman)
            info "Using Podman (requested explicitly)"
            ensure_podman
            return
            ;;
    esac

    # Auto mode: detect a running runtime first.
    if detect_container_runtime; then
        if [ "$CONTAINER_RUNTIME" = "podman" ]; then
            ensure_podman_socket
        fi
        return
    fi

    # Neither runtime running — try to start an installed-but-stopped one.
    if command -v docker &>/dev/null; then
        warn "Docker is installed but not running"
        if is_alpine; then
            rc-service docker start 2>/dev/null || true
        else
            systemctl start docker 2>/dev/null || true
        fi
        if docker info &>/dev/null; then
            CONTAINER_RUNTIME="docker"
            CONTAINER_SOCKET="/var/run/docker.sock"
            ok "Docker started"
            return
        fi
    fi

    if command -v podman &>/dev/null; then
        warn "Podman is installed but not running"
        ensure_podman_socket
        if podman info &>/dev/null; then
            CONTAINER_RUNTIME="podman"
            CONTAINER_SOCKET="/run/podman/podman.sock"
            ok "Podman started"
            return
        fi
    fi

    # Nothing installed — ask which to install.
    info "No container runtime detected."
    echo ""
    echo "  Which container runtime should Icefall use?"
    echo ""
    echo "    1) Docker          — widest compatibility"
    echo "    2) Podman          — daemonless, rootless-capable"
    echo "    3) Auto-detect     — recommended, pick for me"
    echo ""

    local choice
    if [ "$NONINTERACTIVE" = "--yes" ]; then
        choice="3"
    else
        read -rp "Install [1/2/3]: " choice
    fi

    case "$choice" in
        1)
            install_docker
            ;;
        2)
            install_podman
            ;;
        *)
            # Auto / unsure: default to Docker for a fresh box (widest
            # compatibility), matching the prior behavior.
            info "Auto-detect: no runtime present — installing Docker"
            install_docker
            ;;
    esac
}

install_docker() {
    info "Installing Docker..."
    curl -fsSL https://get.docker.com | sh

    if is_alpine; then
        rc-update add docker default 2>/dev/null || true
        rc-service docker start 2>/dev/null || true
    else
        systemctl enable docker
        systemctl start docker
    fi

    if ! docker info &>/dev/null; then
        error "Docker installed but failed to start"
    fi

    CONTAINER_RUNTIME="docker"
    CONTAINER_SOCKET="/var/run/docker.sock"
    ok "Docker installed and verified"
}

install_podman() {
    info "Installing Podman..."
    case "$OS_ID" in
        ubuntu|debian)
            apt-get update &>/dev/null
            apt-get install -y podman &>/dev/null
            ;;
        fedora)
            dnf install -y podman &>/dev/null
            ;;
        centos|rhel|rocky|almalinux)
            dnf install -y podman &>/dev/null || yum install -y podman &>/dev/null
            ;;
        alpine)
            apk add --no-cache podman &>/dev/null
            ;;
        *)
            error "Cannot auto-install Podman for $OS_ID. Install manually: https://podman.io/docs/installation"
            ;;
    esac

    ensure_podman_socket

    if ! podman info &>/dev/null; then
        error "Podman installed but failed to start"
    fi

    CONTAINER_RUNTIME="podman"
    CONTAINER_SOCKET="/run/podman/podman.sock"
    ok "Podman installed and verified"
}

install_caddy() {
    if command -v caddy &>/dev/null; then
        ok "Caddy $(caddy version 2>/dev/null | head -1 || echo '(version unknown)')"
        if is_alpine; then
            rc-service caddy status &>/dev/null || rc-service caddy start 2>/dev/null || true
        else
            systemctl is-active --quiet caddy || systemctl start caddy 2>/dev/null || true
        fi

        local caddy_ok=false
        for _ in 1 2 3; do
            if curl -sf http://localhost:2019/config/ &>/dev/null; then
                caddy_ok=true; break
            fi
            sleep 1
        done
        if $caddy_ok; then
            ok "Caddy admin API reachable"
        else
            warn "Caddy admin API not yet reachable at localhost:2019 — may need manual config"
        fi
        return
    fi

    info "Installing Caddy..."
    case "$OS_ID" in
        ubuntu|debian)
            apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl &>/dev/null
            curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg 2>/dev/null
            curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
            apt-get update &>/dev/null
            apt-get install -y caddy &>/dev/null
            ;;
        fedora)
            dnf install -y 'dnf-command(copr)' &>/dev/null
            dnf copr enable -y @caddy/caddy &>/dev/null
            dnf install -y caddy &>/dev/null
            ;;
        centos|rhel|rocky|almalinux)
            dnf install -y 'dnf-command(copr)' &>/dev/null || yum install -y yum-plugin-copr &>/dev/null
            dnf copr enable -y @caddy/caddy &>/dev/null || true
            dnf install -y caddy &>/dev/null || yum install -y caddy &>/dev/null
            ;;
        alpine)
            apk add --no-cache caddy &>/dev/null
            ;;
        *)
            warn "Cannot auto-install Caddy for $OS_ID. Install manually: https://caddyserver.com/docs/install"
            return
            ;;
    esac

    if is_alpine; then
        rc-update add caddy default 2>/dev/null || true
        rc-service caddy start 2>/dev/null || true
    else
        systemctl enable caddy
        systemctl start caddy
    fi
    ok "Caddy installed"
}

install_icefall() {
    if [ -f "$ICEFALL_BIN" ]; then
        local current_version
        current_version=$("$ICEFALL_BIN" --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        if [ "$ICEFALL_VERSION" != "latest" ] && [ "$current_version" = "${ICEFALL_VERSION#v}" ]; then
            ok "Icefall $current_version already installed (matches requested version)"
            return
        fi
        info "Upgrading Icefall from $current_version..."
    fi

    info "Installing Icefall ($ARCH)..."

    local download_url
    if [ "$ICEFALL_VERSION" = "latest" ]; then
        download_url="https://github.com/nickbevers/icefall/releases/latest/download/icefall-${ARCH}-unknown-linux-gnu"
    else
        download_url="https://github.com/nickbevers/icefall/releases/download/${ICEFALL_VERSION}/icefall-${ARCH}-unknown-linux-gnu"
    fi

    if command -v curl &>/dev/null; then
        curl -fsSL "$download_url" -o "${ICEFALL_BIN}.tmp"
    else
        wget -qO "${ICEFALL_BIN}.tmp" "$download_url"
    fi

    chmod 755 "${ICEFALL_BIN}.tmp"
    mv "${ICEFALL_BIN}.tmp" "$ICEFALL_BIN"
    ok "Binary installed to $ICEFALL_BIN"
}

setup_config() {
    if [ -f "$ICEFALL_CONFIG" ]; then
        ok "Config already exists at $ICEFALL_CONFIG (not overwriting)"
        return
    fi

    info "Creating configuration..."
    mkdir -p /etc/icefall
    mkdir -p "$ICEFALL_DATA"

    local encryption_key
    encryption_key=$(openssl rand -base64 32)

    cat > "$ICEFALL_CONFIG" << EOF
listen_addr = "0.0.0.0"
listen_port = 3000
data_dir = "$ICEFALL_DATA"
sqlite_path = "$ICEFALL_DATA/icefall.db"
runtime = "$CONTAINER_RUNTIME"
container_socket = "$CONTAINER_SOCKET"
caddy_admin_url = "http://localhost:2019"
encryption_key = "$encryption_key"
log_level = "info"
pid_file = "/var/run/icefall.pid"

# base_domain = "apps.example.com"
EOF

    ok "Config written to $ICEFALL_CONFIG"
}

setup_service() {
    if is_alpine; then
        setup_openrc
    else
        setup_systemd
    fi
}

setup_systemd() {
    info "Configuring systemd service..."

    local runtime_dep=""
    local runtime_after="network.target caddy.service"
    if [ "$CONTAINER_RUNTIME" = "podman" ]; then
        runtime_dep="Requires=podman.socket"
        runtime_after="network.target podman.socket caddy.service"
    else
        runtime_dep="Requires=docker.service"
        runtime_after="network.target docker.service caddy.service"
    fi

    cat > "$ICEFALL_SERVICE" << EOF
[Unit]
Description=Icefall Deployment Platform
After=$runtime_after
$runtime_dep

[Service]
Type=notify
ExecStart=/usr/local/bin/icefall daemon start
ExecStopPost=-/var/lib/icefall/updates/icefall.rollback rollback --check
Restart=on-failure
RestartSec=2
StartLimitBurst=3
StartLimitIntervalSec=300
WatchdogSec=60
KillMode=mixed
TimeoutStopSec=30
Environment=ICEFALL_CONFIG=/etc/icefall/config.toml

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable icefall
    ok "Systemd service configured"
}

setup_openrc() {
    info "Configuring OpenRC service..."
    local init_script="/etc/init.d/icefall"

    local rc_dep="docker"
    if [ "$CONTAINER_RUNTIME" = "podman" ]; then
        rc_dep="podman"
    fi

    cat > "$init_script" << EOF
#!/sbin/openrc-run
name="icefall"
description="Icefall Deployment Platform"
command="/usr/local/bin/icefall"
command_args="daemon start"
command_background="yes"
pidfile="/var/run/icefall.pid"
depend() {
    need net $rc_dep
    after caddy
}
EOF

    chmod 755 "$init_script"
    rc-update add icefall default 2>/dev/null || true
    ok "OpenRC service configured"
}

start_services() {
    info "Starting services..."

    if is_alpine; then
        rc-service caddy status &>/dev/null || rc-service caddy start 2>/dev/null || true
        rc-service icefall start 2>/dev/null || true
    else
        systemctl is-active --quiet caddy || systemctl start caddy 2>/dev/null || true
        systemctl start icefall
    fi

    ok "Icefall daemon running"
}

print_success() {
    local server_ip
    server_ip=$(curl -s https://api.ipify.org 2>/dev/null || hostname -I 2>/dev/null | awk '{print $1}' || echo "localhost")

    echo ""
    echo "============================================"
    echo ""
    info "Icefall is installed and running!"
    echo ""
    echo "  Dashboard: http://${server_ip}:3000"
    echo "  Runtime:   $CONTAINER_RUNTIME ($CONTAINER_SOCKET)"
    echo "  Config:    $ICEFALL_CONFIG"
    echo "  Data:      $ICEFALL_DATA"
    if is_alpine; then
        echo "  Logs:      cat /var/log/icefall.log"
    else
        echo "  Logs:      journalctl -u icefall -f"
    fi
    echo "  Install:   $ICEFALL_LOG"
    echo ""
    echo "  Next: Open the dashboard to create your admin account."
    echo ""
    echo "============================================"
}

main() {
    mkdir -p "$(dirname "$ICEFALL_LOG")"
    echo "--- Icefall install started $(date -u) ---" >> "$ICEFALL_LOG"
    echo ""
    info "Icefall Installer"
    echo ""

    check_root
    detect_os
    detect_arch
    check_prereqs
    install_caddy
    install_icefall
    setup_config
    setup_service
    start_services
    print_success
}

main "$@"
