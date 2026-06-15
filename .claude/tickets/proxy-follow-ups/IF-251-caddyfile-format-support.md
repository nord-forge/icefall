# IF-251: Caddyfile format support in advanced mode

**Phase:** Proxy follow-ups (post-IF-149 backlog)
**Priority:** Low — likely won't-do
**Estimate:** M

## Description

IF-149's advanced mode edits Caddy's **JSON** config exclusively, matching Icefall's design decision to use the Caddy JSON admin API everywhere. Some users author config in the more concise **Caddyfile** syntax and have asked to paste a Caddyfile instead. This ticket tracks that request.

**Status: likely won't-do.** Filed to record the deferral from IF-149. Supporting Caddyfile means adapting/parsing it to JSON (Caddy's `adapt` endpoint can do this), which adds a parsing surface and a second source of truth. Confirm there is real demand before scheduling.

## Acceptance Criteria (sketch — refine before scheduling)

- [ ] Advanced mode offers a JSON / Caddyfile toggle.
- [ ] Caddyfile input is adapted to JSON (via Caddy's `/adapt` endpoint) before validation/apply.
- [ ] Validation errors map back to the Caddyfile the user wrote, not the adapted JSON.
- [ ] Round-trip story documented (config is stored/applied as JSON; the Caddyfile is input-only or stored alongside).
- [ ] Scoped per-app like the JSON path — never applies a whole-server Caddyfile.

## Out of Scope

- Storing the canonical config as a Caddyfile (JSON remains the source of truth).
- Caddyfile-only features without a JSON equivalent.

## Dependencies

- IF-149 (reverse proxy management UI)

## Notes

Decision needed before scheduling: is this worth the added parsing surface, or close as "JSON-only by design"?
