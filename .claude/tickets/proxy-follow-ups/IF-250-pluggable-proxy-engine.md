# IF-250: Pluggable reverse-proxy engine (Traefik / Nginx)

**Phase:** Proxy follow-ups (post-IF-149 backlog)
**Priority:** Low — speculative
**Estimate:** XL

## Description

Icefall is currently hard-wired to Caddy (JSON admin API) for reverse proxying. Some operators standardize on Traefik or Nginx and would prefer Icefall drive their existing proxy rather than run Caddy. This ticket explores abstracting the proxy layer behind a trait so an alternative engine can be plugged in.

**Status: speculative.** File now so the deferral from IF-149 is tracked; do not schedule without a concrete user request. May ultimately be a "won't do" if Caddy stays the only supported engine.

## Acceptance Criteria (sketch — refine before scheduling)

- [ ] Define a `ReverseProxy` trait abstracting the operations Icefall needs (add/update/remove route, balanced routes, validate, apply, full-config read).
- [ ] Caddy becomes one implementation of the trait (no behaviour change).
- [ ] At least one alternative implementation (Traefik **or** Nginx) behind a config flag.
- [ ] Feature parity matrix documenting which IF-149 presets/advanced-mode features each engine supports.
- [ ] Migration path / docs for switching engines on an existing install.

## Out of Scope

- Running multiple proxy engines simultaneously.
- Auto-migrating existing Caddy config to the alternative engine's format.

## Dependencies

- IF-149 (reverse proxy management UI) — establishes the proxy operations to abstract.
- Phase 31 (load balancing) — balanced-route generation must also be abstracted.

## Notes

Decision needed before scheduling: is multi-engine support a real product goal, or should this be closed as "Caddy-only by design"?
