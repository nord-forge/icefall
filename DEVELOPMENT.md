# Icefall — Local Development Guide

## Prerequisites

- **Rust** (stable toolchain): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Bun**: `curl -fsSL https://bun.sh/install | bash`
- **Docker Desktop**: running and accessible via `/var/run/docker.sock`
- **Git**: for cloning repos during builds

## Quick start

```bash
# 1. Clone the repo
git clone https://github.com/nord-forge/icefall.git
cd icefall

# 2. Build the Rust daemon
cargo build

# 3. Generate a config with a random encryption key
cargo run -- init
# Accept defaults — config is written to ~/Library/Application Support/icefall/config.toml

# 4. Start the daemon
cargo run -- daemon start
# The API is now running at http://localhost:3000

# 5. In a second terminal, start the dashboard dev server
cd dashboard
bun install
bun dev
# Dashboard at http://localhost:4321 — proxies API calls to :3000
```

Open http://localhost:4321 — you'll see the onboarding wizard.

## What works locally

| Feature | Local | Notes |
|---------|-------|-------|
| Dashboard UI | ✅ | Full functionality |
| Onboarding wizard | ✅ | Create admin, server check, skip domain |
| App creation | ✅ | Create apps via UI or CLI |
| Docker builds | ✅ | Requires Docker Desktop running |
| Container deploys | ✅ | Apps run as Docker containers |
| Managed databases | ✅ | Postgres, MySQL, Redis, MongoDB as containers |
| Health checks | ✅ | TCP checks on local containers |
| Logs | ✅ | Captured from container stdout/stderr |
| Env vars | ✅ | Encrypted in SQLite |
| SSE live updates | ✅ | Real-time build progress |
| HTTPS / custom domains | ❌ | Needs a public server + real domain |
| Webhook deploys | ❌ | Needs a public URL for GitHub/GitLab to reach |
| Install script | ❌ | Designed for Linux servers, not macOS |

## Development workflow

### Backend (Rust)

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy

# Run the daemon in foreground
cargo run -- daemon start

# The API is at http://localhost:3000/api/v1/
# OpenAPI spec at http://localhost:3000/api/v1/openapi.json
```

### Dashboard (Astro + Preact)

```bash
cd dashboard

# Install dependencies
bun install

# Dev server with hot reload (proxies /api to :3000)
bun dev

# Build for production
bun run build
# Output in dashboard/dist/ — served by the Rust daemon in production
```

### Docs site (Starlight)

```bash
cd website

# Install dependencies
bun install

# Dev server
bun dev

# Build
bun run build
```

## Testing a deploy locally

```bash
# 1. Start the daemon (if not already running)
cargo run -- daemon start

# 2. Open the dashboard at localhost:4321
# 3. Click "New App"
# 4. Enter a public GitHub repo URL (e.g., https://github.com/withastro/astro-starter)
# 5. Click Deploy
# 6. Watch the build progress in real-time

# Or via CLI:
cargo run -- login
# Server URL: http://localhost:3000
# Token: (create one from the dashboard Settings > Tokens)

cargo run -- apps list
cargo run -- status
```

## Testing with a database

```bash
# Via dashboard: Databases > Add Database > PostgreSQL > Create
# Or via CLI:
cargo run -- db create postgres
# Outputs the connection string

cargo run -- db list
```

## Project structure

```
icefall/
├── src/                    # Rust daemon
│   ├── api/                # Axum HTTP routes
│   ├── build/              # Framework detection, Dockerfile generation, build orchestration
│   ├── deploy/             # Container deploy, health checks, preview environments
│   ├── docker/             # Docker Engine client (Bollard)
│   ├── caddy/              # Caddy admin API client
│   ├── db/                 # SQLite database, models, migrations
│   ├── events/             # SSE event bus
│   ├── monitoring/         # Health runner, metrics, log storage, backups
│   ├── cli/                # CLI commands
│   └── config/             # TOML config system
├── dashboard/              # Astro + Preact web UI
│   └── src/
│       ├── islands/        # Preact interactive components
│       ├── pages/          # Astro page shells
│       ├── layouts/        # Dashboard + onboarding layouts
│       ├── styles/         # Shared CSS (tokens, forms, layout)
│       ├── lib/            # API client, SSE client, types, formatters
│       └── stores/         # Nanostores (theme, apps, server)
├── website/                # Starlight documentation site
├── assets/brand/           # Logo, favicon, brand guidelines
├── install.sh              # One-liner server install script
└── .claude/                # Tickets, board, design screenshots
```

## Environment variables

The daemon reads config from `config.toml` but these env vars override:

| Variable | Default | Description |
|----------|---------|-------------|
| `ICEFALL_PORT` | `3000` | API listen port |
| `ICEFALL_DATA_DIR` | `/var/lib/icefall` | Data directory |
| `ICEFALL_DOCKER_SOCKET` | `/var/run/docker.sock` | Docker socket path |
| `ICEFALL_LOG_LEVEL` | `info` | Log verbosity |
| `ICEFALL_ENCRYPTION_KEY` | (from config) | Base64-encoded 32-byte key |
| `ICEFALL_CONFIG` | (auto-detected) | Config file path override |

## Testing on a real server

For the full experience (HTTPS, domains, webhooks), use a cheap VPS:

```bash
# On a fresh Ubuntu 24.04 server:
curl -fsSL https://raw.githubusercontent.com/nord-forge/icefall/main/install.sh | bash

# Then open http://YOUR_IP:3000 to complete setup
```

Minimum: 1 vCPU, 1GB RAM, 10GB disk. Recommended: 2 vCPU, 4GB RAM.
