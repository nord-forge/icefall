# IF-249: Per-path middleware for proxy presets

**Phase:** Proxy follow-ups (post-IF-149 backlog)
**Priority:** Low
**Estimate:** L

## Description

IF-149 applies middleware presets (rate limit, basic auth, headers, redirects) **per app** only — a single set of middleware covers every route the app serves. This ticket extends presets so middleware can target a specific path prefix (e.g. basic-auth on `/admin`, a stricter rate limit on `/api`), while leaving the rest of the app untouched.

The `BasicAuthPreset` already carries an unused optional `path` field as a placeholder; this ticket generalizes the concept across presets.

## Acceptance Criteria

- [ ] Presets can be scoped to a path prefix; an unset path means "whole app" (current behaviour, preserved).
- [ ] UI: each preset can optionally specify a path matcher; clear indication of app-wide vs path-scoped.
- [ ] Caddy generation emits path-matched routes/subroutes ordered so the most specific match wins.
- [ ] Multiple presets on overlapping paths have well-defined, documented precedence.
- [ ] Read-only viewer shows which middleware applies to which path.
- [ ] Backward compatible: existing app-wide presets keep working with no migration.

## Out of Scope

- Path-based routing to different upstreams (already handled by IF-069 path-based routing).
- Regex path matchers (prefix matching only for v1).

## Dependencies

- IF-149 (reverse proxy management UI)
- IF-069 (path-based routing) — provides the per-path route separation this builds on.
