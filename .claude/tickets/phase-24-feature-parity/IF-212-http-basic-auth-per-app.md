# IF-212: HTTP basic auth per application

**Phase:** 24 — Feature Parity
**Priority:** Low
**Estimate:** S

## Description

Allow enabling HTTP basic auth on any application as a quick access-control layer. Useful for staging environments, internal tools, or pre-launch sites that shouldn't be publicly accessible. Implemented at the Caddy reverse proxy level — no app code changes needed.

## Acceptance Criteria

- [ ] Toggle in app settings: "Enable HTTP Basic Auth"
- [ ] When enabled: username + password fields (password stored encrypted)
- [ ] Caddy route updated to include `basicauth` directive when enabled
- [ ] Password hashed with bcrypt for Caddy's `basicauth` format
- [ ] Auth applies to all routes for the app (no path-level granularity)
- [ ] Works with both container and static-site deploy modes
- [ ] Preview deployments inherit the basic auth setting (or can be toggled independently)
- [ ] API: `PUT /apps/{id}` accepts `basic_auth_enabled`, `basic_auth_username`, `basic_auth_password`
- [ ] For multi-server: Caddy config update propagated to the worker agent

## Technical Notes

- Caddy's `basicauth` directive handles the challenge/response — just generate the Caddyfile block
- Store the bcrypt hash, not the plaintext password (generate via `bcrypt` crate)
- Consider allowing multiple username/password pairs in a future iteration

## Out of Scope

- IP-based access control (separate feature)
- Per-path auth rules
- Integration with OAuth/SSO for app-level auth

## Dependencies

- IF-005 (Caddy admin API client)
- IF-023 (Domain management — route configuration)
