# IF-149: Reverse proxy management UI

**Phase:** 22 — Expansion (v1.2)
**Priority:** Medium
**Estimate:** M

## Description

Expose Caddy's proxy configuration in the dashboard with a read-only viewer and a guarded advanced editing mode. Users can inspect the auto-generated routes, apply middleware presets (rate limiting, basic auth, redirect rules, custom headers), and reset to defaults if they break something. The goal is transparency without footguns — most users should never need to touch raw config.

## Acceptance Criteria

### Read-Only Config Viewer

- [ ] New "Proxy" tab on the app detail page (between Domains and Settings)
- [ ] Shows the auto-generated Caddy JSON config for this app's routes in a syntax-highlighted code block
- [ ] Read-only by default — no edit capability until user explicitly enables advanced mode
- [ ] Displays active routes: upstream address, TLS status, matched domains, matched paths
- [ ] Shows which middleware is active per route (rate limit, basic auth, headers, redirects)
- [ ] "Copy config" button for the raw JSON

### Middleware Presets

- [ ] Presets are predefined Caddy config snippets that users can toggle on/off per app
- [ ] **Rate limiting**: requests per second/minute with burst, per-IP or global
  - UI: toggle + rate input (e.g., "100 requests per minute") + burst input
  - Caddy: `rate_limit` module (or `respond 429` with `client_ip` matcher if module not available)
- [ ] **Basic auth**: username/password protection for the entire app or specific paths
  - UI: toggle + username + password fields + optional path restriction
  - Caddy: `basicauth` directive with bcrypt-hashed password
  - Warning: "This is HTTP-level auth, not application auth"
- [ ] **Redirect rules**: source path → destination URL with status code (301/302)
  - UI: list of redirect rules with add/remove
  - Caddy: `redir` directive
- [ ] **Custom response headers**: add/override response headers
  - UI: key-value list with add/remove
  - Caddy: `header` directive
  - Pre-populated security headers as suggestions (X-Frame-Options, CSP, etc.)
- [ ] **Force HTTPS**: redirect all HTTP to HTTPS (on by default via Caddy, but show toggle)
- [ ] Each preset shows a brief explanation of what it does and when to use it
- [ ] Presets are stored in the app's configuration (SQLite), not directly in Caddy config
- [ ] On save: Icefall regenerates the full Caddy config for this app including all active presets

### Advanced Mode

- [ ] "Enable Advanced Mode" button with confirmation dialog: "Editing raw proxy config can break routing. Changes are validated before applying."
- [ ] Textarea/code editor with the full Caddy JSON config for this app
- [ ] Syntax validation before applying — reject invalid JSON and invalid Caddy config structure
- [ ] Test button: "Validate" sends the config to Caddy's `/validate` endpoint without applying
- [ ] Apply button: sends the config to Caddy's `/load` endpoint
- [ ] If the config breaks routing (Caddy returns error): show the error, do not apply, keep previous config
- [ ] "Reset to Default" button: regenerates the standard Icefall config for this app, discarding all custom edits
- [ ] When advanced mode is active, preset toggles are disabled (raw config takes precedence)
- [ ] `has_custom_proxy_config` flag on the app model — when true, Icefall skips auto-regeneration on deploy

### Global Proxy Settings

- [ ] New section in the global Settings page: "Reverse Proxy"
- [ ] Shows Caddy version and status (running/stopped)
- [ ] Global default headers that apply to all routes
- [ ] Global rate limit defaults
- [ ] "View Full Config" button: shows the complete Caddy config (all apps) in read-only mode
- [ ] "Reload Caddy" button: triggers a full config reload (admin only)

### API Endpoints

- [ ] `GET /apps/{id}/proxy` — get proxy config for an app (auto-generated or custom)
- [ ] `PUT /apps/{id}/proxy/presets` — update middleware presets for an app
- [ ] `PUT /apps/{id}/proxy/custom` — save custom proxy config (enables advanced mode)
- [ ] `POST /apps/{id}/proxy/validate` — validate a proxy config without applying
- [ ] `POST /apps/{id}/proxy/reset` — reset to auto-generated config
- [ ] `GET /settings/proxy` — get global proxy settings
- [ ] `PUT /settings/proxy` — update global proxy defaults
- [ ] `POST /settings/proxy/reload` — reload Caddy config (admin only)

### Safety

- [ ] Config backups: before any proxy change, store the previous config in a `proxy_config_history` table (keep last 10)
- [ ] Rollback: if a config change breaks routing, "Undo Last Change" restores from history
- [ ] Deploy behavior: when an app with `has_custom_proxy_config = false` is redeployed, proxy config is regenerated (including presets). When `true`, the custom config is preserved.
- [ ] Role enforcement: viewer can see config, deployer can toggle presets, admin can use advanced mode

## Technical Notes

- Caddy's admin API at `localhost:2019` provides `/config/`, `/validate`, and `/load` endpoints — all JSON-based
- The existing `CaddyClient` (IF-005) already handles route CRUD — extend it for full config read/write
- Rate limiting in Caddy may require the `caddy-ratelimit` plugin — check if it's included in the standard build, otherwise use a `respond 429` approach with a request counter
- For multi-server: proxy config is per-server — the control plane sends the config to the agent, which applies it to the local Caddy instance
- Syntax highlighting: use a lightweight JSON highlighter (no heavy editor library for read-only mode)

## Out of Scope

- Switching reverse proxy (Caddy → Traefik/Nginx)
- Load balancer configuration (v2.0 with multi-server)
- Custom Caddy modules/plugins installation
- Caddyfile format (Icefall uses Caddy's JSON API exclusively)
- Per-path middleware (only per-app for now — path-based routing from IF-069 handles path separation)

## Dependencies

- IF-005 (Caddy admin API client)
- IF-069 (Path-based routing)
- IF-023 (Domain management)
