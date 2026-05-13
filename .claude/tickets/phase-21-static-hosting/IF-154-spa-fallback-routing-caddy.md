# IF-154: SPA fallback routing in Caddy file_server

**Phase:** 21 — Static Hosting Expansion
**Priority:** High
**Estimate:** S
**Dependencies:** None

## Description

SPAs (React Router, Vue Router, SvelteKit static, etc.) use client-side routing. When a user navigates to `/dashboard/settings` and refreshes, Caddy needs to serve `index.html` instead of returning 404. The current `CaddyRoute::file_server()` implementation needs to include a `try_files` rewrite so that requests for non-existent paths fall back to `index.html`.

Verify the current Caddy route config handles this, and if not, add the try_files / rewrite directive.

## Acceptance Criteria

### Caddy route config in `src/caddy/types.rs`
- [ ] `file_server` route includes a `rewrite` handler or `try_files` that serves `{path} /index.html`
- [ ] Static assets (JS, CSS, images, fonts) are served directly without rewrite
- [ ] 404 pages: if the app has a `404.html`, serve it for genuinely missing routes
- [ ] Works correctly for both root-level and subdirectory deployments

### Tests
- [ ] Verify Caddy route JSON includes try_files / rewrite configuration
- [ ] Test that `/some/spa/route` serves index.html
- [ ] Test that `/assets/main.js` serves the actual file

## Out of Scope

- SSG routes (pre-rendered HTML for each path) — those already work with file_server
- Custom rewrite rules per-app

## Files to Modify

- `src/caddy/types.rs` — update `CaddyRoute::file_server()` to include try_files
- `src/caddy/routes.rs` — if route construction needs updating
