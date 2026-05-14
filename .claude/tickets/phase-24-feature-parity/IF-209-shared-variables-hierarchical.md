# IF-209: Shared variables (hierarchical env var inheritance)

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** M

## Description

Support shared variables at project and server scopes that cascade downward to apps. Variables defined once at a higher scope are available to all resources within that scope unless overridden. Useful for shared secrets (Stripe keys, SMTP credentials, S3 access keys) used by multiple apps.

## Acceptance Criteria

- [ ] `shared_variables` table: `id`, `scope` (project/server), `scope_id` (project_id or server_id), `key`, `value` (encrypted), `is_sensitive`, `created_at`, `updated_at`
- [ ] Shared variables UI page (sidebar nav item) with scope tabs: Project, Server
- [ ] Per-scope variable list with add/edit/delete
- [ ] Normal view (form fields) and developer view (raw `.env` textarea with import/export)
- [ ] Variable resolution order: app > environment (if IF-147 done) > project > server
- [ ] In app env var editor: show which vars are inherited with provenance badge ("from project: my-project")
- [ ] Deploy pipeline resolves shared variables at deploy time, merging with app-level vars
- [ ] API endpoints: `GET/POST /shared-variables/{scope}/{scope_id}`, `PUT/DELETE /shared-variables/{id}`
- [ ] Encrypted at rest (reuse existing AES-256-GCM from IF-014)

## Technical Notes

- Resolution during deploy: load all applicable shared vars by scope chain, then overlay app vars
- Team-level scope deferred until Teams feature (v2.0)

## Out of Scope

- Team-level shared variables (depends on v2.0 teams/multi-tenancy)
- Magic variable interpolation / auto-generation
- Cross-project variable sharing

## Dependencies

- IF-014 (Environment variable management)
- IF-074 (Projects — needed for project-scope)
- IF-117 (Servers table — needed for server-scope)
