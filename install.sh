#!/usr/bin/env bash
# Icefall Installation Script
# Usage: curl -fsSL https://icefall.dev/install.sh | bash
#
# Environment variables:
#   ICEFALL_VERSION    - Version to install (default: latest)
#   ICEFALL_RUNTIME    - Container runtime: docker | podman | auto (default: auto)
#   ICEFALL_GITHUB_ORG - GitHub org to install from (default: nord-forge)
#   ICEFALL_REPO       - Full "org/repo" override (default: $ICEFALL_GITHUB_ORG/icefall)
#   NO_COLOR           - Disable colored output
#
# Flags (positional, any order):
#   --yes             - Non-interactive; accept defaults
#   --runtime=NAME    - Container runtime: docker | podman | auto
#   --port=N          - Dashboard listen port (default 3000)
#   --domain=NAME     - Base domain for the dashboard + apps (e.g. apps.example.com)

set -euo pipefail

ICEFALL_VERSION="${ICEFALL_VERSION:-latest}"
ICEFALL_REPO="${ICEFALL_REPO:-${ICEFALL_GITHUB_ORG:-nord-forge}/icefall}"
ICEFALL_BIN="/usr/local/bin/icefall"
ICEFALL_DATA="/var/lib/icefall"
ICEFALL_CONFIG="/etc/icefall/config.toml"
ICEFALL_SERVICE="/etc/systemd/system/icefall.service"
ICEFALL_LOG="/var/log/icefall-install.log"

# Populated by detect_memory(): total RAM (MB) and whether to enable low-memory
# mode. On a small box (<= 1.5 GB) Icefall shrinks its SQLite cache and buffers
# and the systemd unit gets a tighter MemoryMax.
TOTAL_RAM_MB=0
LOW_MEMORY="false"
# RAM (MB) at or below which low-memory mode is enabled automatically. Override
# with ICEFALL_LOW_MEMORY=true|false to force.
LOW_MEMORY_THRESHOLD_MB="${ICEFALL_LOW_MEMORY_THRESHOLD_MB:-1536}"

# Runtime preference: docker | podman | auto. Env var is the default; a
# --runtime= flag overrides it. "auto" (or empty) means detect/prompt.
RUNTIME_CHOICE="${ICEFALL_RUNTIME:-auto}"
NONINTERACTIVE=""

# When the script is run via `curl ... | bash`, stdin is the pipe (the script
# text), not the user's keyboard — so `read` would hit EOF and every prompt
# would fall through silently. We instead read from the controlling terminal
# (/dev/tty) when one is attached. HAVE_TTY records whether that's possible;
# if there's no tty (true headless / CI), we behave as if --yes was passed.
HAVE_TTY=""
if [ -e /dev/tty ] && { exec 3<>/dev/tty; } 2>/dev/null; then
    HAVE_TTY="1"
    exec 3>&-
fi

# Dashboard port and base domain. Env vars seed the defaults; flags override;
# otherwise we prompt (interactive) or fall back to the default (--yes).
# LISTEN_PORT_SET / BASE_DOMAIN_SET track whether the value came from a flag/env
# so the interactive prompts know whether to ask.
LISTEN_PORT="${ICEFALL_PORT:-3000}"
LISTEN_PORT_SET=""
[ -n "${ICEFALL_PORT:-}" ] && LISTEN_PORT_SET="1"
BASE_DOMAIN="${ICEFALL_BASE_DOMAIN:-}"
BASE_DOMAIN_SET=""
[ -n "${ICEFALL_BASE_DOMAIN:-}" ] && BASE_DOMAIN_SET="1"

for _arg in "$@"; do
    case "$_arg" in
        --yes)
            NONINTERACTIVE="--yes"
            ;;
        --runtime=*)
            RUNTIME_CHOICE="${_arg#--runtime=}"
            ;;
        --port=*)
            LISTEN_PORT="${_arg#--port=}"
            LISTEN_PORT_SET="1"
            ;;
        --domain=*)
            BASE_DOMAIN="${_arg#--domain=}"
            BASE_DOMAIN_SET="1"
            ;;
    esac
done

# Validate the port now so a bad flag fails fast, before any install work.
case "$LISTEN_PORT" in
    ''|*[!0-9]*) echo "Invalid --port / ICEFALL_PORT value: '$LISTEN_PORT' (expected a number 1-65535)" >&2; exit 1 ;;
esac
if [ "$LISTEN_PORT" -lt 1 ] || [ "$LISTEN_PORT" -gt 65535 ]; then
    echo "Invalid --port / ICEFALL_PORT value: '$LISTEN_PORT' (expected 1-65535)" >&2; exit 1
fi

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

# True when we can and should prompt the user: not forced non-interactive, and
# a controlling terminal is attached (works under `curl | bash`, where stdin is
# the pipe). Centralizes the check so every prompt site agrees.
interactive() {
    [ "$NONINTERACTIVE" != "--yes" ] && [ -n "$HAVE_TTY" ]
}

# Read a line from the controlling terminal (/dev/tty), not stdin — stdin is the
# piped script under `curl | bash`. Usage: prompt VAR "Prompt text: "
# Returns non-zero (and leaves VAR empty) if no tty is available.
prompt() {
    local __var="$1" __msg="$2" __reply=""
    [ -n "$HAVE_TTY" ] || { printf -v "$__var" '%s' ""; return 1; }
    # -e for the escape codes in the prompt; read from the terminal explicitly.
    read -r -p "$(printf '%b' "$__msg")" __reply < /dev/tty || true
    printf -v "$__var" '%s' "$__reply"
}

confirm() {
    if ! interactive; then return 0; fi
    local response
    prompt response "$1 [y/N] "
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

# Detect total RAM and decide whether to enable low-memory mode. An explicit
# ICEFALL_LOW_MEMORY=true|false forces the choice; otherwise it's automatic
# below LOW_MEMORY_THRESHOLD_MB. Also sizes the systemd MemoryMax later.
detect_memory() {
    # MemTotal is in kB; fall back to 0 if /proc/meminfo is unavailable.
    local mem_kb
    mem_kb=$(awk '/^MemTotal:/ {print $2; exit}' /proc/meminfo 2>/dev/null || echo 0)
    TOTAL_RAM_MB=$(( mem_kb / 1024 ))

    case "${ICEFALL_LOW_MEMORY:-auto}" in
        1|true|yes|on)   LOW_MEMORY="true" ;;
        0|false|no|off)  LOW_MEMORY="false" ;;
        *)
            if [ "$TOTAL_RAM_MB" -gt 0 ] && [ "$TOTAL_RAM_MB" -le "$LOW_MEMORY_THRESHOLD_MB" ]; then
                LOW_MEMORY="true"
            else
                LOW_MEMORY="false"
            fi
            ;;
    esac

    if [ "$TOTAL_RAM_MB" -gt 0 ]; then
        if [ "$LOW_MEMORY" = "true" ]; then
            info "Detected ${TOTAL_RAM_MB} MB RAM — enabling low-memory mode"
        else
            info "Detected ${TOTAL_RAM_MB} MB RAM"
        fi
    fi
}

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
            # Ensure the compose CLI and cgroup delegation are present even when
            # Podman was already installed by hand.
            command -v podman-compose &>/dev/null || install_podman_compose
            setup_cgroup_delegation
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
# Print an annotated Docker-vs-Podman comparison. On a small box the daemonless
# nature of Podman is the deciding factor, so the callout is louder there.
print_runtime_tradeoffs() {
    echo ""
    echo -e "  ${BOLD}Docker${RESET}"
    echo "    + Widest compatibility — the default most images/tooling assume."
    echo "    + Mature, familiar; easiest to debug with existing docs."
    echo "    - Runs an always-on daemon (dockerd + containerd) that idles at"
    echo "      ~60-100 MB RAM — a real cost on a 1 GB server."
    echo ""
    echo -e "  ${BOLD}Podman${RESET} (rootful — what this installer sets up)"
    echo "    + Daemonless — no always-on process; ~0 MB idle. The runtime cost"
    echo "      scales with running containers, not a fixed floor."
    echo "    + Drop-in Docker-compatible API; Icefall supports it fully."
    echo "    + Installer adds podman-compose (Raw Compose mode) and sets up"
    echo "      cgroups-v2 delegation, so resource limits work."
    echo "    - Slightly less ubiquitous; a few images expect the Docker socket"
    echo "      path (Icefall handles this, but third-party tooling may not)."
    echo "    - Rootless Podman is supported too, but can't enforce limits without"
    echo "      delegation and can't bind ports < 1024 — rootful avoids both."
    if [ "$LOW_MEMORY" = "true" ]; then
        echo ""
        warn "This looks like a small server (${TOTAL_RAM_MB} MB RAM)."
        echo -e "  ${BOLD}${YELLOW}Recommended: Podman${RESET} — its daemonless design frees ~80 MB"
        echo "  of RAM for your app containers. Pick Docker only if you specifically"
        echo "  need Docker-only tooling."
    fi
    echo ""
}

prompt_runtime_choice() {
    # Only relevant in interactive auto mode.
    [ "$RUNTIME_CHOICE" = "auto" ] || return 0
    interactive || return 0

    local have_docker=false have_podman=false
    command -v docker &>/dev/null && have_docker=true
    command -v podman &>/dev/null && have_podman=true

    # Only prompt when there is an actual choice to make.
    if [ "$have_docker" = true ] && [ "$have_podman" = true ]; then
        info "Both Docker and Podman are installed."
        print_runtime_tradeoffs
        echo "  Which container runtime should Icefall use?"
        echo ""
        echo "    1) Docker"
        echo "    2) Podman"
        echo "    3) Auto-detect     — pick for me"
        echo ""
        # On a small box, recommend Podman as the default for an empty answer.
        local default_choice="3" prompt_hint="[1/2/3]"
        if [ "$LOW_MEMORY" = "true" ]; then
            default_choice="2"; prompt_hint="[1/2/3, default 2=Podman]"
        fi
        local choice
        prompt choice "Use $prompt_hint: "
        choice="${choice:-$default_choice}"
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

    # Nothing installed — ask which to install. On a small box the choice has
    # real memory implications, so we surface the trade-offs and recommend
    # Podman, while always leaving the final pick to the user.
    info "No container runtime detected."
    print_runtime_tradeoffs
    echo "  Which container runtime should Icefall install?"
    echo ""
    echo "    1) Docker          — widest compatibility"
    echo "    2) Podman          — daemonless, lighter idle RAM"
    echo "    3) Auto-detect     — pick for me"
    echo ""

    # Default for an empty answer (and for non-interactive installs):
    # Podman on a small box, Docker otherwise.
    local default_choice="3"
    if [ "$LOW_MEMORY" = "true" ]; then
        default_choice="2"
    fi

    local choice
    if ! interactive; then
        choice="$default_choice"
        if [ "$LOW_MEMORY" = "true" ]; then
            info "Non-interactive small-server install — choosing Podman (lighter idle RAM)"
        fi
    else
        local hint="[1/2/3]"
        [ "$LOW_MEMORY" = "true" ] && hint="[1/2/3, default 2=Podman]"
        prompt choice "Install $hint: "
        choice="${choice:-$default_choice}"
    fi

    case "$choice" in
        1)
            install_docker
            ;;
        2)
            install_podman
            ;;
        *)
            # Auto / unsure: Docker on a normal box (widest compatibility),
            # Podman on a small box (the daemonless RAM win is decisive there).
            if [ "$LOW_MEMORY" = "true" ]; then
                info "Auto-detect: small server — installing Podman (daemonless, lighter idle RAM)"
                install_podman
            else
                info "Auto-detect: no runtime present — installing Docker"
                install_docker
            fi
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

# Install the compose CLI for Podman so Raw Compose mode (deploy_mode =
# "raw-compose") works out of the box. Best-effort: a missing package is a
# warning, not a fatal error — only raw-compose deploys need it.
install_podman_compose() {
    info "Installing podman-compose (for Raw Compose mode)..."
    local ok_compose=false
    case "$OS_ID" in
        ubuntu|debian)
            apt-get install -y podman-compose &>/dev/null && ok_compose=true ;;
        fedora|centos|rhel|rocky|almalinux)
            { dnf install -y podman-compose &>/dev/null || yum install -y podman-compose &>/dev/null; } && ok_compose=true ;;
        alpine)
            apk add --no-cache podman-compose &>/dev/null && ok_compose=true ;;
    esac
    # Fall back to pip if the distro has no package (common on older releases).
    if ! $ok_compose && command -v pip3 &>/dev/null; then
        pip3 install --quiet podman-compose &>/dev/null && ok_compose=true
    fi
    if $ok_compose || command -v podman-compose &>/dev/null; then
        ok "podman-compose available"
    else
        warn "Could not install podman-compose automatically. Raw Compose mode will"
        warn "be unavailable until you install it (e.g. 'pip3 install podman-compose')."
    fi
}

# Enable cgroups-v2 controller delegation so resource limits (memory/CPU) are
# enforced even for rootless Podman. Without this the kernel silently ignores
# limits on rootless containers. Rootful is unaffected. Best-effort, non-fatal.
setup_cgroup_delegation() {
    # Only meaningful on a cgroups-v2 (unified) host.
    [ -f /sys/fs/cgroup/cgroup.controllers ] || return 0
    # Delegate the cpu/memory controllers to user slices via a systemd drop-in.
    local dropin="/etc/systemd/system/user@.service.d/icefall-delegate.conf"
    if [ ! -f "$dropin" ] && command -v systemctl &>/dev/null; then
        mkdir -p "$(dirname "$dropin")"
        cat > "$dropin" << 'EOF'
# Added by Icefall: delegate cpu/memory cgroup controllers to user sessions so
# rootless Podman can enforce container resource limits.
[Service]
Delegate=cpu cpuset io memory pids
EOF
        systemctl daemon-reload 2>/dev/null || true
        ok "Enabled cgroups-v2 delegation (rootless resource limits)"
    fi
}

install_podman() {
    info "Installing Podman (rootful)..."
    # Icefall installs and uses ROOTFUL Podman by default: it enforces container
    # resource limits without extra setup and can publish privileged ports.
    # Rootless Podman is fully supported too — point ICEFALL_CONTAINER_SOCKET at
    # the per-user socket — but on rootless, limits need cgroups-v2 delegation
    # (set up below) and ports < 1024 aren't publishable.
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

    install_podman_compose
    setup_cgroup_delegation
    ensure_podman_socket

    if ! podman info &>/dev/null; then
        error "Podman installed but failed to start"
    fi

    CONTAINER_RUNTIME="podman"
    CONTAINER_SOCKET="/run/podman/podman.sock"
    ok "Podman installed and verified (rootful)"
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

# Fetch a URL to stdout, preferring curl, falling back to wget.
fetch() {
    if command -v curl &>/dev/null; then
        curl -fsSL "$1"
    else
        wget -qO- "$1"
    fi
}

# Download a URL to a file, preferring curl, falling back to wget.
fetch_to() {
    if command -v curl &>/dev/null; then
        curl -fsSL "$1" -o "$2"
    else
        wget -qO "$2" "$1"
    fi
}

# Resolve "latest" to a concrete release tag (e.g. v1.2.0) via the GitHub API.
resolve_release_tag() {
    if [ "$ICEFALL_VERSION" != "latest" ]; then
        # Normalize to a leading "v" (accept both "1.2.0" and "v1.2.0").
        case "$ICEFALL_VERSION" in
            v*) echo "$ICEFALL_VERSION" ;;
            *)  echo "v$ICEFALL_VERSION" ;;
        esac
        return
    fi

    local tag
    tag=$(fetch "https://api.github.com/repos/${ICEFALL_REPO}/releases/latest" 2>/dev/null \
        | grep -m1 '"tag_name"' \
        | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
    if [ -z "$tag" ]; then
        error "Could not resolve the latest Icefall release from github.com/${ICEFALL_REPO}. Set ICEFALL_VERSION=vX.Y.Z to pin a version."
    fi
    echo "$tag"
}

install_icefall() {
    local tag
    tag="$(resolve_release_tag)"
    local version="${tag#v}"

    if [ -f "$ICEFALL_BIN" ]; then
        local current_version
        current_version=$("$ICEFALL_BIN" --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        if [ "$current_version" = "$version" ]; then
            ok "Icefall $current_version already installed (matches $tag)"
            return
        fi
        info "Upgrading Icefall from $current_version to $version..."
    else
        info "Installing Icefall $version ($ARCH)..."
    fi

    # Release artifacts are tarballs containing just the icefall binary (the
    # dashboard is embedded in it). Naming/layout: scripts/sign-release.py + release.yml.
    local arch_label="${ARCH}-linux"
    local tarball="icefall-${tag}-${arch_label}.tar.gz"
    local base_url="https://github.com/${ICEFALL_REPO}/releases/download/${tag}"

    local workdir
    workdir="$(mktemp -d)"
    trap 'rm -rf "$workdir"' RETURN

    info "Downloading $tarball..."
    fetch_to "${base_url}/${tarball}" "${workdir}/${tarball}" \
        || error "Failed to download $tarball from $base_url"

    # Verify SHA-256 against the published .sha256 sidecar.
    if fetch_to "${base_url}/${tarball}.sha256" "${workdir}/${tarball}.sha256" 2>/dev/null; then
        ( cd "$workdir" && sha256sum -c "${tarball}.sha256" >/dev/null 2>&1 ) \
            || error "SHA-256 checksum verification failed for $tarball"
        ok "Checksum verified"
    else
        warn "No checksum file published for $tarball — skipping verification"
    fi

    info "Extracting..."
    tar -xzf "${workdir}/${tarball}" -C "$workdir"

    # Locate the binary (top level, or one directory deep).
    local extracted_bin
    extracted_bin="$(find "$workdir" -maxdepth 2 -type f -name icefall ! -name '*.tar.gz' | head -1)"
    [ -n "$extracted_bin" ] || error "No 'icefall' binary found inside $tarball"

    install -m 755 "$extracted_bin" "$ICEFALL_BIN"
    ok "Binary installed to $ICEFALL_BIN"
    # The dashboard is embedded in the binary (IF-255) — nothing else to install.
}

# Best-effort public IP of this server, used for the DNS instructions. Falls
# back to the first local address, then a placeholder.
SERVER_IP=""
detect_public_ip() {
    if [ -n "$SERVER_IP" ]; then return 0; fi
    SERVER_IP=$(curl -fsS --max-time 5 https://api.ipify.org 2>/dev/null || true)
    [ -z "$SERVER_IP" ] && SERVER_IP=$(curl -fsS --max-time 5 https://ifconfig.me 2>/dev/null || true)
    [ -z "$SERVER_IP" ] && SERVER_IP=$(hostname -I 2>/dev/null | awk '{print $1}')
    [ -z "$SERVER_IP" ] && SERVER_IP="<your-server-ip>"
}

# Interactive setup: confirm the dashboard port and (optionally) the base
# domain before anything is written. Flags/env that were already supplied are
# respected and not re-prompted. Under --yes we keep the defaults silently.
prompt_setup() {
    # Port: only ask when interactive and not already set by flag/env.
    if [ -z "$LISTEN_PORT_SET" ] && interactive; then
        local answer
        prompt answer "Dashboard port [${LISTEN_PORT}]: "
        answer="${answer:-$LISTEN_PORT}"
        case "$answer" in
            ''|*[!0-9]*) warn "Not a number — keeping ${LISTEN_PORT}" ;;
            *)
                if [ "$answer" -ge 1 ] && [ "$answer" -le 65535 ]; then
                    LISTEN_PORT="$answer"
                else
                    warn "Out of range — keeping ${LISTEN_PORT}"
                fi
                ;;
        esac
    fi
    info "Dashboard port: ${LISTEN_PORT}"

    # Base domain: only ask when interactive and not already set. Empty answer
    # means "no domain" — the dashboard is reached by IP:port for now.
    if [ -z "$BASE_DOMAIN_SET" ] && interactive; then
        echo ""
        info "A base domain lets Icefall serve the dashboard and your apps over"
        info "HTTPS (e.g. https://apps.example.com). Leave blank to skip for now"
        info "and reach the dashboard by IP:port — you can set it later."
        local answer
        prompt answer "Base domain (blank to skip): "
        BASE_DOMAIN="$(echo "$answer" | tr -d '[:space:]')"
    fi
}

# Print the exact DNS records the user must create, and (interactively) offer to
# wait until the domain resolves to this server before finishing.
guide_dns() {
    [ -n "$BASE_DOMAIN" ] || return 0
    detect_public_ip

    # Choose A (IPv4) vs AAAA (IPv6) by the shape of the detected address.
    local record_type="A"
    case "$SERVER_IP" in
        *:*) record_type="AAAA" ;;
    esac

    echo ""
    echo -e "  ${BOLD}DNS records to create for ${BASE_DOMAIN}:${RESET}"
    echo ""
    echo "    ${record_type}      ${BASE_DOMAIN}        ->  ${SERVER_IP}"
    echo "    ${record_type}      *.${BASE_DOMAIN}      ->  ${SERVER_IP}    (wildcard, for per-app subdomains)"
    echo ""
    echo "  The wildcard lets each app get its own subdomain automatically."
    echo "  Caddy provisions HTTPS certificates once DNS points here and ports"
    echo "  80 and 443 are reachable from the internet."
    echo ""

    if ! interactive; then
        return 0
    fi
    if ! confirm "Wait now until ${BASE_DOMAIN} resolves to ${SERVER_IP}?"; then
        info "Skipping DNS verification — set the records when ready."
        return 0
    fi

    info "Waiting for DNS… (Ctrl-C to stop waiting; install continues regardless)"
    local resolver=""
    if command -v dig &>/dev/null; then
        resolver="dig"
    elif command -v host &>/dev/null; then
        resolver="host"
    elif command -v nslookup &>/dev/null; then
        resolver="nslookup"
    else
        warn "No dig/host/nslookup available to verify DNS — skipping the wait."
        return 0
    fi

    local resolved="" attempts=0
    while [ "$attempts" -lt 60 ]; do
        case "$resolver" in
            dig)      resolved=$(dig +short "$BASE_DOMAIN" 2>/dev/null | tail -1) ;;
            host)     resolved=$(host "$BASE_DOMAIN" 2>/dev/null | awk '/has .* address/ {print $NF; exit}') ;;
            nslookup) resolved=$(nslookup "$BASE_DOMAIN" 2>/dev/null | awk '/^Address: / {print $2; exit}') ;;
        esac
        if [ "$resolved" = "$SERVER_IP" ]; then
            ok "${BASE_DOMAIN} now resolves to ${SERVER_IP}"
            return 0
        fi
        attempts=$((attempts + 1))
        sleep 5
    done
    warn "${BASE_DOMAIN} did not resolve to ${SERVER_IP} within ~5 min."
    warn "Install will finish anyway; HTTPS will be issued once DNS propagates."
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

    # Emit base_domain only when the user provided one; otherwise leave a
    # commented example so the dashboard is reached by IP:port until set.
    local base_domain_line="# base_domain = \"apps.example.com\""
    if [ -n "$BASE_DOMAIN" ]; then
        base_domain_line="base_domain = \"$BASE_DOMAIN\""
    fi

    cat > "$ICEFALL_CONFIG" << EOF
listen_addr = "0.0.0.0"
listen_port = $LISTEN_PORT
data_dir = "$ICEFALL_DATA"
sqlite_path = "$ICEFALL_DATA/icefall.db"
runtime = "$CONTAINER_RUNTIME"
container_socket = "$CONTAINER_SOCKET"
caddy_admin_url = "http://localhost:2019"
encryption_key = "$encryption_key"
log_level = "info"
pid_file = "/var/run/icefall.pid"

# Low-memory mode shrinks the SQLite page cache (~62 MB -> ~16 MB) and a few
# in-memory buffers so Icefall fits on a 1 vCPU / 1 GB server. Auto-enabled by
# the installer below ${LOW_MEMORY_THRESHOLD_MB} MB RAM.
low_memory = $LOW_MEMORY
# Override the SQLite page cache directly (KiB). Omitted = derive from low_memory.
# sqlite_cache_kib = 16000

$base_domain_line
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

    # Cap the daemon's memory so a leak or runaway cache can't OOM the box and
    # take down the apps it manages. The daemon idles ~80-90 MB; give it a
    # comfortable ceiling sized from total RAM, clamped to [256, 768] MB.
    # MemoryHigh (soft) throttles before MemoryMax (hard) kills.
    local mem_cap_mb=512
    if [ "$TOTAL_RAM_MB" -gt 0 ]; then
        mem_cap_mb=$(( TOTAL_RAM_MB / 8 ))
        [ "$mem_cap_mb" -lt 256 ] && mem_cap_mb=256
        [ "$mem_cap_mb" -gt 768 ] && mem_cap_mb=768
    fi
    local mem_high_mb=$(( mem_cap_mb * 4 / 5 ))

    cat > "$ICEFALL_SERVICE" << EOF
[Unit]
Description=Icefall Deployment Platform
After=$runtime_after
$runtime_dep

[Service]
Type=notify
WorkingDirectory=$ICEFALL_DATA
ExecStart=/usr/local/bin/icefall daemon start
ExecStopPost=-/var/lib/icefall/updates/icefall.rollback rollback --check
Restart=on-failure
RestartSec=2
StartLimitBurst=3
StartLimitIntervalSec=300
WatchdogSec=60
KillMode=mixed
TimeoutStopSec=30
# Memory ceiling (IF: low-memory hardening). MemoryHigh throttles, MemoryMax is
# the hard OOM limit. Sized from total RAM; raise if you run many apps and see
# the daemon throttled in \`systemctl status icefall\`.
MemoryHigh=${mem_high_mb}M
MemoryMax=${mem_cap_mb}M
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
directory="$ICEFALL_DATA"
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
    detect_public_ip

    # Prefer the HTTPS domain URL when a base domain was configured; the daemon
    # serves the dashboard route over it. Otherwise fall back to IP:port.
    local dashboard_url="http://${SERVER_IP}:${LISTEN_PORT}"
    if [ -n "$BASE_DOMAIN" ]; then
        dashboard_url="https://${BASE_DOMAIN}"
    fi

    echo ""
    echo "============================================"
    echo ""
    info "Icefall is installed and running!"
    echo ""
    echo "  Dashboard: ${dashboard_url}"
    if [ -n "$BASE_DOMAIN" ]; then
        echo "             (also http://${SERVER_IP}:${LISTEN_PORT} until DNS/HTTPS is ready)"
    fi
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
    if [ -n "$BASE_DOMAIN" ]; then
        echo "  Make sure DNS for ${BASE_DOMAIN} (and *.${BASE_DOMAIN}) points to"
        echo "  ${SERVER_IP}, and ports 80 + 443 are open, so Caddy can issue HTTPS."
        echo ""
    else
        echo "  Tip: open port ${LISTEN_PORT} in your firewall to reach the dashboard."
        echo ""
    fi
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
    detect_memory
    check_prereqs
    # Gather the interactive choices (port, base domain) up front, before the
    # long download/install steps — so the rest of the install runs unattended.
    prompt_setup
    install_caddy
    install_icefall
    setup_config
    setup_service
    start_services
    # With the daemon up, point the user at the DNS records for their domain
    # (and optionally wait until it resolves) so HTTPS can be issued.
    guide_dns
    print_success
}

main "$@"
