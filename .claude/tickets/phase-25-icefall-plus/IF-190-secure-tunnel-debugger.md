# IF-190: Secure Tunnel Debugger

**Phase:** 25 — Icefall+
**Priority:** High
**Estimate:** M

## Description

One-command secure tunnel from your local machine to any deployed container's port — routed through the agent's WebSocket connection. No SSH required, no ports exposed to the internet. `icefall tunnel my-db:5432 --local 5432` gives you a local port forwarding to the remote database.

## Acceptance Criteria

- [ ] CLI command: `icefall tunnel <app-or-db>:<remote-port> [--local <local-port>]`
- [ ] Tunnel established over the existing agent WebSocket connection (no new ports)
- [ ] Local TCP listener on the specified port forwards traffic through the tunnel
- [ ] Works for any TCP service: databases, debug ports, internal APIs
- [ ] Authentication: uses the user's API token or session
- [ ] Encryption: all traffic encrypted via the WebSocket TLS + envelope encryption layer
- [ ] Dashboard: "Tunnel" button on app/database detail → shows the CLI command to copy
- [ ] Multiple simultaneous tunnels supported
- [ ] Tunnel auto-closes after 30 minutes of inactivity (configurable)
- [ ] Status output: "Tunnel active: localhost:5432 → my-db:5432 on server-1"

## Technical Notes

- The agent already has a WebSocket connection and can handle arbitrary binary messages
- Add a `tunnel.open` / `tunnel.data` / `tunnel.close` message type to the agent protocol
- Local side: `tokio::net::TcpListener` on the specified port, relay bytes over WebSocket
- Agent side: `tokio::net::TcpStream::connect` to the container's IP:port, relay bytes back
- This is essentially SSH port forwarding without SSH — simpler, already authenticated

## Dependencies

- IF-119 (Agent WebSocket — transport layer)
- IF-038 (CLI management commands — for the CLI interface)
