# IF-248: Bundle / install the caddy-ratelimit module

**Phase:** Proxy follow-ups (post-IF-149 backlog)
**Priority:** Medium
**Estimate:** M

## Description

IF-149's rate-limiting preset generates real `rate_limit` handler config **only when the running Caddy build includes the `caddy-ratelimit` module**. Standard Caddy builds do not, so Icefall currently falls back to a `respond 429` handler that documents intent but does **not** actually count or throttle requests (see `src/caddy/proxy.rs::rate_limit_handler` and `has_rate_limit_module`).

This ticket makes rate limiting actually enforce by ensuring the module is present in the Caddy that Icefall ships/manages.

## Acceptance Criteria

- [ ] Icefall's bundled/managed Caddy includes the `caddy-ratelimit` module (via `xcaddy` build, a pinned custom image, or documented install step).
- [ ] `CaddyClient::has_rate_limit_module()` returns true on the shipped build; the 429 fallback path is only taken on genuinely unsupported installs.
- [ ] Module version is pinned (no `latest`) and recorded in the build/release process.
- [ ] Docs updated: rate-limiting guide notes the module is included and the fallback no longer applies to default installs.
- [ ] Existing preset → handler generation is unchanged; this is purely making the native path available.

## Out of Scope

- Per-path rate limits (see IF-251 per-path middleware).
- Distributed/shared rate-limit state across multiple servers.

## Dependencies

- IF-149 (reverse proxy management UI) — defines the preset and fallback this replaces.
