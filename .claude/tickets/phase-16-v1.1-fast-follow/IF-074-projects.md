# IF-074: Projects (resource grouping)

**Phase:** 16 — v1.1 Fast Follow
**Priority:** Medium
**Estimate:** M

## Description

Organizational structure for users with many apps. A project groups related apps and databases together. This is grouping only — no environment promotion, cloning, or environment-scoped secrets.

## Acceptance Criteria

### Project Management
- [ ] New page: `/projects` — list all projects
- [ ] Create project: name (required), description (optional), color/icon (optional)
- [ ] Edit project: rename, update description
- [ ] Delete project: moves all resources back to "Unassigned" (does not delete apps/databases)
- [ ] Default project: "Personal" auto-created for existing resources

### App / Database Assignment
- [ ] App settings tab: "Project" dropdown to assign app to a project
- [ ] Database settings: "Project" dropdown to assign database to a project
- [ ] New apps/databases default to "Unassigned" unless created from within a project context

### Navigation
- [ ] Sidebar: group apps by project (collapsible sections)
- [ ] Dashboard home: filter by project (dropdown or tabs)
- [ ] Project detail page: list all apps and databases in the project with status overview

### Backend
- [ ] New `projects` table: `id`, `name`, `description`, `color`, `created_at`, `updated_at`
- [ ] Migration to add `project_id` foreign key (nullable) to `apps` and `databases` tables
- [ ] Migration to create default "Personal" project and assign all existing resources
- [ ] API endpoints:
  - `GET /api/v1/projects` — list projects
  - `POST /api/v1/projects` — create project
  - `GET /api/v1/projects/{id}` — get project with resources
  - `PUT /api/v1/projects/{id}` — update project
  - `DELETE /api/v1/projects/{id}` — delete project (unassign resources)
- [ ] `GET /api/v1/apps` supports optional `?project_id=` filter

### General
- [ ] Light and dark theme verified
- [ ] Mobile responsive

## Technical Notes

- Simple foreign key relationship — no complex hierarchy
- The sidebar in `dashboard/src/components/sidebar/Sidebar.astro` needs to fetch and group apps
- Consider a nanostore for project state: `dashboard/src/stores/projects.ts`

## Out of Scope

- Environments within projects (v1.2 — IF separate ticket)
- Project-level environment variables
- Project-level notification settings
- Project templates / cloning
- Project-level access control (all team members see all projects)

## Dependencies

- IF-017 (dashboard home), IF-019 (app detail page)
