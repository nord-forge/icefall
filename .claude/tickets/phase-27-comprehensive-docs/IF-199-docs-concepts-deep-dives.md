# IF-199: Concepts deep-dive docs

**Phase:** 27 — Comprehensive Docs
**Priority:** High
**Estimate:** L

## Description

Write comprehensive concept pages that explain HOW Icefall works under the hood. These are the pages that make developers trust the platform — they answer "what happens when I click deploy?" with precision.

## Pages to Create/Rewrite

### `concepts/architecture.mdx` (rewrite)
- [ ] System architecture diagram (binary, SQLite, Docker, Caddy, agent)
- [ ] Request lifecycle: HTTP request → Caddy → container → response
- [ ] Deploy lifecycle: trigger → clone → detect → build → swap → health check
- [ ] Data flow: where state lives (SQLite), where config lives (Caddy), where containers live (Docker)
- [ ] Single-server vs multi-server architecture comparison
- [ ] Resource footprint: idle memory, CPU, disk usage

### `concepts/builds.mdx` (rewrite)
- [ ] Framework detection: how Icefall identifies your stack
- [ ] Dockerfile generation: what gets generated and why
- [ ] Multi-stage builds: builder vs runtime stages
- [ ] Build caching: Docker layer cache strategy
- [ ] Custom Dockerfile: when and how to use your own
- [ ] Build environment variables vs runtime environment variables
- [ ] Monorepo builds (base directory)
- [ ] Native static site builds (no Docker)

### `concepts/deployments.mdx` (new)
- [ ] Blue-green deployment strategy explained
- [ ] Zero-downtime swap: how Caddy routes switch
- [ ] Health check gating: container must pass before routing
- [ ] Rollback: what happens, what's preserved, what's lost
- [ ] Deploy triggers: manual, webhook, API, scheduled, MCP
- [ ] Deploy statuses: pending, building, deploying, running, failed, rolled_back

### `concepts/networking.mdx` (new)
- [ ] Docker networking: bridge networks, container DNS
- [ ] Caddy reverse proxy: automatic HTTPS, route generation
- [ ] Domain routing: how domains map to containers
- [ ] Path-based routing: how multiple services share a domain
- [ ] Internal URLs: service-to-service communication
- [ ] Port mapping: container ports vs external ports
- [ ] Cloudflare Tunnel: how it bypasses NAT

### `concepts/security.mdx` (new)
- [ ] Authentication: sessions, API tokens, OAuth
- [ ] Authorization: admin/deployer/viewer roles
- [ ] Encryption at rest: AES-256-GCM for secrets
- [ ] Encryption in transit: TLS everywhere, envelope encryption for remote deploys
- [ ] Container isolation: namespaces, cgroups, resource limits
- [ ] Agent security: X25519 keypairs, token-based auth, WebSocket TLS
- [ ] Secret management: how env vars are stored, transferred, and injected

### `concepts/multi-server.mdx` (new)
- [ ] Control plane vs worker architecture
- [ ] Agent binary: what it does, how it connects
- [ ] Enrollment flow: from "Add Server" to first deploy
- [ ] Server selection: recommendation scoring algorithm
- [ ] App migration: how apps move between servers
- [ ] Offline handling: what happens when a server goes down
- [ ] Envelope encryption: how env vars are transferred securely

### `concepts/environments.mdx` (rewrite)
- [ ] Environment variable scoping: app, environment, project
- [ ] Variable resolution cascade: project → environment → app
- [ ] Secret masking in UI and logs
- [ ] .env import format
- [ ] Reserved keys (PORT, HOST, etc.)

### `concepts/databases.mdx` (new)
- [ ] Managed database provisioning: what Icefall creates (container, volume, credentials)
- [ ] Supported databases and versions
- [ ] Backup strategy: schedule, retention, S3 upload
- [ ] Linking: how connection strings are injected as env vars
- [ ] Database browser: how it connects safely
- [ ] Scaling considerations

### `concepts/container-runtime.mdx` (new)
- [ ] Docker vs Podman: what Icefall supports and why
- [ ] How Icefall talks to the runtime (bollard, socket API)
- [ ] Runtime auto-detection during installation
- [ ] Docker-specific behavior vs Podman-specific behavior
- [ ] Networking differences: Docker bridge vs Podman netavark
- [ ] When to choose Docker vs Podman (decision guide)
- [ ] Rootful vs rootless: what's supported, what's coming

## Standards

- [ ] Every concept page includes a diagram or visual
- [ ] Technical details backed by code references (not marketing claims)
- [ ] Comparison callouts: "Unlike X, Icefall does Y because Z"
- [ ] Each page is self-contained (no mandatory reading order)
- [ ] Docker/Podman differences called out explicitly wherever behavior diverges

## Dependencies

- IF-047 (Documentation site)
- IF-206 (Podman runtime support)
