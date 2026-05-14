# IF-183: Ghost Mode — zero-downtime container hibernation

**Phase:** 26 — Icefall+
**Priority:** High
**Estimate:** L

## Description

Automatically suspend idle containers to free resources and wake them on the first incoming request. The Rust reverse proxy holds the connection open during cold start (typically 1-3 seconds), then proxies the request through. Users see a brief delay, not an error. Enables running 10+ side projects on a $5 VPS without keeping them all in memory.

## Acceptance Criteria

- [ ] Per-app setting: "Enable Ghost Mode" toggle with idle timeout (default: 30 minutes, min: 5 min)
- [ ] Idle detection: the agent tracks last-request timestamp per container. When idle exceeds threshold, stop the container.
- [ ] Wake-on-request: Caddy route returns a "please wait" interstitial or holds the TCP connection while Icefall starts the container
- [ ] Container start time tracked; if >10 seconds, notify user that Ghost Mode may cause poor UX for this app
- [ ] Status indicator: app card shows "hibernating" state with a moon icon
- [ ] Wake manually: "Wake" button on the app overview
- [ ] Excluded from Ghost Mode: apps with active WebSocket connections, databases, Compose stacks
- [ ] Health check runs after wake before routing traffic
- [ ] SSE event: `app.hibernated` and `app.woken` for notification dispatch
- [ ] Dashboard: "Ghost Mode savings" metric showing estimated RAM saved

## Technical Notes

- The Caddy `reverse_proxy` with a `lb_try_duration` can hold while the container starts, but a custom Caddy plugin or a Rust middleware in front of Caddy may be needed for the interstitial page
- Alternative: use Caddy's `respond` directive with a meta-refresh while the container starts, then redirect
- Container stop uses graceful stop (Docker `docker stop` / Podman `podman stop`), not kill
- Container start uses the existing start command — no rebuild needed since the container exists
- For multi-server: the agent on each server manages hibernation locally

## Dependencies

- IF-004 (Container runtime client — Docker/Podman)
- IF-026 (Container metrics — for idle detection)
