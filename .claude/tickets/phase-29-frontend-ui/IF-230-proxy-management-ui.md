# IF-230: Reverse proxy management UI

**Phase:** 29 — Frontend UI
**Priority:** Medium
**Estimate:** M

## Description

Build the proxy config viewer and middleware preset UI (IF-149). Users can inspect auto-generated Caddy routes, toggle middleware presets, and optionally edit raw config.

## Acceptance Criteria

- [ ] App detail: new "Proxy" tab
- [ ] Read-only config viewer: syntax-highlighted Caddy JSON
- [ ] Middleware presets section (toggle cards):
  - Rate limiting: toggle + rate input + burst
  - Basic auth: toggle + username/password (uses IF-212 backend)
  - Redirect rules: source → destination list
  - Custom headers: key-value list
- [ ] "Advanced Mode" button with confirmation dialog
- [ ] Advanced: full config editor textarea with validate + apply
- [ ] "Reset to Default" button
- [ ] Global proxy settings in Settings page: Caddy version, reload button
- [ ] Config history with "Undo Last Change"
- [ ] a11y: code viewer accessible, form validation

## Dependencies

- IF-149 (Proxy management backend)
