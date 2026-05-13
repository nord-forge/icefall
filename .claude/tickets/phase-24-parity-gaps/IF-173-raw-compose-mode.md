# IF-173: Raw Compose mode

**Phase:** 24 — Parity Gaps
**Priority:** Medium
**Estimate:** S

## Description

Add a "Raw Compose" mode that passes the docker-compose.yml directly to Docker Compose with minimal Icefall intervention. For advanced users who need Compose features that Icefall's managed Compose pipeline doesn't support (custom networking, build args, profiles, extends).

## Acceptance Criteria

- [ ] New deploy mode: "Raw Compose" alongside "Managed Compose" in app settings
- [ ] When raw mode is enabled:
  - Icefall does NOT parse or modify the Compose file
  - Icefall runs `docker compose -f docker-compose.yml up -d` directly
  - Environment variables from the app's env var editor are passed via `--env-file`
  - Icefall still manages: domain routing (Caddy), deploy history, log capture, start/stop/restart
- [ ] Deploy log shows the raw `docker compose` output
- [ ] Warning in settings: "Raw Compose mode gives you full control but disables Icefall's managed networking, health checks, and blue-green deploys for this stack."
- [ ] Health check: Icefall checks if the expected containers are running (by Compose project name), not individual health endpoints
- [ ] Stop/restart: uses `docker compose stop/restart` instead of individual container commands

## Technical Notes

- Use `tokio::process::Command` to shell out to `docker compose` — bollard doesn't support Compose natively
- The Compose project name should be `icefall-{app-slug}` to namespace containers
- Capture stdout/stderr and stream via SSE to the deploy log
- For multi-server: the agent executes `docker compose` commands on the worker

## Dependencies

- IF-073 (Docker Compose support — managed mode)
