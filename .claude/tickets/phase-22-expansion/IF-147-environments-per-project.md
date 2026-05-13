# IF-147: Environments per project

**Phase:** 22 — Expansion (v1.2)
**Priority:** High
**Estimate:** L

## Description

Add environment support within projects so users can organize apps into production, staging, and development tiers. Each environment has its own set of scoped variables that inherit from the project level with per-environment overrides. Apps are assigned to exactly one environment within their project. Variable resolution follows a three-level cascade: project → environment → app.

## Acceptance Criteria

### Database

- [ ] New `environments` table: `id`, `project_id` (FK), `name`, `slug`, `color` (OKLCH string for UI badge), `sort_order`, `created_at`, `updated_at`
- [ ] New `environment_variables` table: `id`, `environment_id` (FK), `key`, `value` (encrypted), `is_secret` (boolean), `created_at`, `updated_at`
- [ ] `apps` table gains `environment_id` (FK, nullable — null = unassigned)
- [ ] Migration seeds three default environments per existing project: Production, Staging, Development
- [ ] Unique constraint on `(project_id, slug)`
- [ ] Unique constraint on `(environment_id, key)` in environment_variables
- [ ] Cascade delete: deleting an environment nullifies `apps.environment_id`, deletes its variables

### API Endpoints

- [ ] `POST /projects/{project_id}/environments` — create environment (admin/deployer)
- [ ] `GET /projects/{project_id}/environments` — list environments for a project
- [ ] `PUT /environments/{id}` — update name, color, sort_order
- [ ] `DELETE /environments/{id}` — delete (cannot delete if it's the only environment with running apps — warn first)
- [ ] `PUT /apps/{id}/environment` — assign app to environment (`{ "environment_id": "..." }`)
- [ ] `GET /environments/{id}/variables` — list environment variables
- [ ] `POST /environments/{id}/variables` — create variable
- [ ] `PUT /environments/{id}/variables/{var_id}` — update variable
- [ ] `DELETE /environments/{id}/variables/{var_id}` — delete variable
- [ ] `GET /apps/{id}/resolved-variables` — returns the merged variable set (project → environment → app) with source annotation per key

### Variable Resolution

- [ ] Three-level cascade: project variables < environment variables < app variables
- [ ] A key set at app level overrides the same key at environment level
- [ ] A key set at environment level overrides the same key at project level
- [ ] `resolved-variables` endpoint returns each variable with a `source` field: `"project"`, `"environment"`, or `"app"`
- [ ] Deploy pipeline uses the resolved variable set, not raw app variables
- [ ] Changing an environment variable triggers a "needs redeploy" indicator on affected apps (no auto-restart)

### Dashboard UI

- [ ] Project detail page shows environment tabs (Production / Staging / Development / custom)
- [ ] Each environment tab lists its apps with status indicators
- [ ] Environment badge (colored pill) on AppCard when viewing project
- [ ] Environment variables editor on environment detail — same UX as app env var editor (masked values, click-to-reveal, .env import)
- [ ] "Resolved Variables" read-only view on app detail showing the merged set with source badges
- [ ] App settings tab: environment assignment dropdown
- [ ] Environment settings: rename, reorder, change color, delete (with confirmation)
- [ ] Empty state when environment has no apps: "Move or create an app in this environment"

### Constraints

- [ ] Environment names: 1-50 chars, alphanumeric + hyphens + spaces
- [ ] Maximum 10 environments per project
- [ ] Default environments (Production/Staging/Development) can be renamed but not auto-deleted
- [ ] Viewer role: read-only access to environments and variables (values masked)
- [ ] Deployer role: can assign apps, manage variables
- [ ] Admin role: can create/delete environments

## Technical Notes

- The variable resolution should be computed at deploy time, not cached — keeps it simple and avoids cache invalidation bugs
- The `resolved-variables` endpoint is read-only and computed on the fly by merging the three layers
- Environment colors should use the existing OKLCH token system (e.g., production = red-ish, staging = amber, dev = blue)
- Consider adding `environment_id` to the deploy record so historical deploys show which environment they ran against

## Out of Scope

- Environment promotion workflows (deploy staging → production)
- Environment cloning (duplicate an environment with all its apps)
- Cross-environment variable diffing
- Environment-scoped domains (e.g., `staging.example.com` auto-assigned)
- Environment-scoped resource limits

## Dependencies

- IF-074 (Projects — resource grouping)
- IF-014 (Environment variable management)
