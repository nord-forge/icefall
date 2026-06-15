# Proxy follow-ups (backlog)

> **Status: BACKLOG** — unscheduled. Spun out of IF-149's "Out of Scope" section.
> Dependencies: IF-149 (reverse proxy management UI) — shipped in PR #84.

## Overview

IF-149 deferred five items. Load-balancer config is already covered by **Phase 31 (load balancing, shipped)** and needs no ticket. The remaining four are tracked here.

## Tickets

| Ticket | Title | Priority | Notes |
|--------|-------|----------|-------|
| [IF-248](IF-248-caddy-ratelimit-plugin-install.md) | Bundle the caddy-ratelimit module | Medium | Upgrades IF-149's 429 fallback to real rate-limit enforcement. Most actionable. |
| [IF-249](IF-249-per-path-middleware.md) | Per-path middleware for presets | Low | Natural IF-149 extension (currently per-app only). Depends on IF-069. |
| [IF-250](IF-250-pluggable-proxy-engine.md) | Pluggable proxy engine (Traefik/Nginx) | Low | Speculative — decide product intent before scheduling; may be won't-do. |
| [IF-251](IF-251-caddyfile-format-support.md) | Caddyfile format in advanced mode | Low | Likely won't-do — conflicts with JSON-only design. Confirm demand first. |

## Suggested order

IF-248 first (concrete value, improves a shipped feature) → IF-249 (extension) → IF-250 / IF-251 only if a real user request lands.
