# IF-073: Docker Compose support

**Phase:** 16 — v1.1 Fast Follow
**Priority:** High
**Estimate:** L

## Description

Add the ability to deploy multi-container stacks from Docker Compose files. This is the most-requested missing feature after Docker image deploy. Many popular self-hosted apps (Supabase, Plausible with ClickHouse, WordPress + MySQL) require multiple containers. Start with a known-good subset of Compose features rather than full spec compliance.

## Acceptance Criteria

### App Creation Flow
- [ ] New deployment type option: "Deploy from Docker Compose" (alongside Git and Docker Image)
- [ ] Compose input options:
  - Upload / paste a `docker-compose.yml` file
  - Git repo URL containing a `docker-compose.yml` (auto-detected)
- [ ] Preview: parse and display the services that will be created before deployment
- [ ] Environment variables: show all `${VAR}` references found in the compose file, allow setting values

### Deployment Pipeline
- [ ] Parse `docker-compose.yml` using a Compose spec parser
- [ ] Supported Compose features (v1.0 of Compose support):
  - `services` with `image`, `build`, `ports`, `environment`, `volumes`, `depends_on`, `restart`
  - `volumes` (named volumes)
  - `networks` (auto-created isolated bridge network per stack)
  - `command` and `entrypoint` overrides
  - Variable interpolation (`${VAR}` and `${VAR:-default}`)
- [ ] Create an isolated bridge network per stack (services communicate via service names)
- [ ] Deploy services in dependency order (respecting `depends_on`)
- [ ] Caddy auto-connected to the stack network for routing
- [ ] Each service gets a container with labels: `icefall.app`, `icefall.stack`, `icefall.service`

### App Detail Page
- [ ] Stack view: show all services within the compose stack
- [ ] Per-service status, logs, start/stop/restart
- [ ] Compose file viewer/editor in settings
- [ ] Stack-level actions: deploy all, stop all, restart all

### Raw Compose Mode
- [ ] Advanced option: "Raw mode" — pass compose file directly to `docker compose up` with minimal Icefall intervention
- [ ] Raw mode skips Icefall's service parsing and just manages the lifecycle
- [ ] Warning: "Raw mode gives you full Compose control but limits Icefall's ability to manage individual services"

### Backend
- [ ] New `stack` or `compose` app type
- [ ] Store compose file content in database (or reference git path)
- [ ] Compose file validation before deployment
- [ ] Handle service updates: detect changes and only recreate modified services

### Unsupported Compose Features (v1.0)
- [ ] Document what's not supported: `extends`, `profiles`, `configs`, `secrets` (Docker secrets), `deploy` (Swarm mode), `build.ssh`, `build.secrets`
- [ ] Show clear error when unsupported features are used

### General
- [ ] Light and dark theme verified
- [ ] Mobile responsive

## Technical Notes

- Consider using the `docker-compose-types` crate for parsing, or `serde_yaml` with a custom Compose struct
- Bollard can create containers with network aliases for service discovery
- The isolated network approach ensures stacks don't interfere with each other
- For the `build` directive: build the image using the existing build pipeline, then deploy
- `depends_on` only controls start order, not readiness — document this limitation

## Out of Scope

- Docker Compose v1 format (only v2/v3)
- `docker compose` CLI passthrough (we manage containers directly via Bollard)
- Service scaling (`scale: 3`) — single instance per service for v1.0
- Compose file includes / extends across files
- Docker Swarm deploy mode

## Dependencies

- IF-065 (Docker image deploy — for the image pull path), IF-064 (volumes UI)
