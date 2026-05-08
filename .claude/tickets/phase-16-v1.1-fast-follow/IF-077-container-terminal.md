# IF-077: Container terminal (browser shell)

**Phase:** 16 — v1.1 Fast Follow
**Priority:** Medium
**Estimate:** M

## Description

Browser-based terminal that exec's into a running container. A "wow" feature for debugging that earns word-of-mouth. Requires a WebSocket TTY proxy with resize handling and per-connection authentication.

## Acceptance Criteria

### App Detail Page — Terminal Tab
- [ ] New tab: "Terminal" in the app detail page
- [ ] Container selection dropdown (if multiple containers exist, e.g., preview envs)
- [ ] Terminal emulator (xterm.js) embedded in the tab:
  - Full ANSI color support
  - Copy/paste support
  - Keyboard shortcuts passthrough (Ctrl+C, etc.)
  - Terminal resize on window/tab resize
  - Scrollback buffer (1000 lines)
- [ ] Default shell: `/bin/sh` (fallback from `/bin/bash` if not available)
- [ ] Connection status indicator: "Connected", "Connecting...", "Disconnected"
- [ ] Reconnect button when disconnected
- [ ] "Close terminal" button to end the session

### Database Detail Page — Terminal Tab
- [ ] Same terminal for database containers
- [ ] Auto-connect to the database CLI:
  - PostgreSQL: `psql -U {user} {database}`
  - MySQL: `mysql -u {user} -p{password} {database}`
  - MongoDB: `mongosh {connection_string}`
  - Redis: `redis-cli`

### Backend
- [ ] WebSocket endpoint: `GET /api/v1/apps/{id}/terminal` (upgrades to WebSocket)
- [ ] Authentication: validate session token or API token before upgrading
- [ ] Role enforcement: admin and deployer can access terminal, viewer gets read-only mode
- [ ] Read-only mode for viewer role: terminal output is visible but input is blocked
- [ ] Docker exec via Bollard:
  - `exec_create` with `AttachStdin`, `AttachStdout`, `AttachStderr`, `Tty`
  - `exec_start` with bidirectional streaming
  - `exec_resize` for terminal size changes
- [ ] Session timeout: auto-disconnect after 30 minutes of inactivity
- [ ] Max concurrent sessions: 5 per container (prevent resource exhaustion)

### Security
- [ ] Per-connection authentication (WebSocket handshake includes auth token)
- [ ] No root shell by default — use the container's configured user
- [ ] Audit log: record terminal sessions (who connected, when, duration)
- [ ] Rate limit: max 10 connection attempts per minute per user

### General
- [ ] Light and dark theme: terminal is always dark (consistent with log viewer)
- [ ] Mobile responsive: terminal should work on tablet (not optimized for phone)

## Technical Notes

- Use `xterm.js` (MIT licensed) for the frontend terminal emulator
- Use `xterm-addon-fit` for automatic resizing
- Use `xterm-addon-web-links` for clickable URLs in terminal output
- Bollard's `exec_create` + `exec_start` returns a stream that can be bridged to WebSocket
- The WebSocket needs to handle bidirectional binary frames (stdin/stdout)
- Consider `tokio-tungstenite` for WebSocket handling (Axum has built-in WebSocket support)

## Out of Scope

- SSH terminal to the host server (container exec only)
- Terminal recording / playback
- Shared terminal sessions (pair programming)
- File upload/download via terminal
- Custom shell selection

## Dependencies

- IF-004 (Docker client), IF-019 (app detail page)
