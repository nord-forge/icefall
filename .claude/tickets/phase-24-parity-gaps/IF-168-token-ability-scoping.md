# IF-168: Granular API token ability scoping

**Phase:** 24 — Parity Gaps
**Priority:** Medium
**Estimate:** M

## Description

Add granular permission scoping to API tokens. Currently tokens inherit the full permissions of the user who created them. Allow users to restrict tokens to specific abilities: read-only, deploy-only, or full access. This enables safe CI/CD tokens that can only deploy, not manage users.

## Acceptance Criteria

- [ ] `abilities` JSON field on the `api_tokens` table (nullable — null = full access for backwards compatibility)
- [ ] Ability scopes:
  - `apps:read` — list and view apps
  - `apps:write` — create, update, delete apps
  - `apps:deploy` — trigger deploys, rollback
  - `databases:read` — list and view databases
  - `databases:write` — create, delete databases
  - `domains:read` / `domains:write`
  - `env:read` / `env:write`
  - `servers:read` / `servers:write`
  - `users:read` / `users:write` (admin only)
  - `settings:read` / `settings:write` (admin only)
- [ ] Token creation form: checkbox list of abilities (default: all checked)
- [ ] Preset buttons: "Full access", "Read only", "Deploy only" (apps:read + apps:deploy + env:read)
- [ ] API middleware: check token abilities against the endpoint being accessed
- [ ] 403 response when a token lacks the required ability: `{ "error": { "code": "insufficient_scope", "message": "Token lacks 'apps:deploy' ability" } }`
- [ ] Token detail view shows granted abilities as badges
- [ ] Existing tokens (null abilities) continue to work as full access

## Technical Notes

- Store abilities as a JSON array: `["apps:read", "apps:deploy", "env:read"]`
- Middleware reads the token's abilities from the session/auth context and checks against a required-ability annotation on each route
- Use an Axum extension or middleware layer for ability checking

## Dependencies

- IF-035 (API token management)
