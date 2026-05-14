# IF-213: Server-level terminal (SSH to host OS)

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** M

## Description

IF-077 provides container-level terminal access via container exec. This ticket adds server-level terminal access — an in-browser shell to the host OS of any connected server. Useful for debugging system issues, checking disk usage, managing the container runtime directly, or troubleshooting network problems.

For the control-plane server: direct shell access. For worker servers: terminal commands proxied through the WebSocket agent.

## Acceptance Criteria

- [ ] Server detail page: "Terminal" tab (alongside Overview/Apps/Metrics/Settings)
- [ ] In-browser xterm.js terminal connected via WebSocket
- [ ] Control plane: shell spawned directly (same mechanism as container terminal but targeting host)
- [ ] Worker servers: terminal input/output proxied through agent WebSocket protocol
- [ ] Agent handles `terminal.server.open`, `terminal.server.input`, `terminal.server.resize`, `terminal.server.close` messages
- [ ] Agent spawns shell as the configured user (from server config, typically the SSH user)
- [ ] Security: require admin role to access server terminal
- [ ] Per-server toggle: "Enable server terminal" in server settings (default: disabled)
- [ ] Optional: require 2FA confirmation before opening server terminal
- [ ] Top-level `/terminal` page: select server → select "server shell" or pick a container
- [ ] Terminal resize works correctly (send SIGWINCH)

## Technical Notes

- On worker: agent spawns a PTY (using `tokio::process::Command` with `pty` crate) and streams I/O over WebSocket
- On control plane: same PTY approach, handled locally
- The terminal should NOT use SSH — use direct PTY spawn (the agent is already on the server)
- Session timeout: auto-close after 30 minutes of inactivity

## Out of Scope

- Terminal session recording / audit log (future feature)
- File transfer through terminal
- Multiple concurrent terminal sessions to the same server

## Dependencies

- IF-077 (Container terminal — UI and WebSocket infrastructure)
- IF-119 (Agent WebSocket endpoint)
- IF-129 (Agent terminal proxy — base protocol)
