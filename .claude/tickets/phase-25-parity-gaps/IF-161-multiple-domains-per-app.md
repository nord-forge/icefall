# IF-161: Multiple domains per app

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** S

## Description

Allow users to add multiple custom domains to a single app. The backend domain model already supports this (domains table has `app_id` FK), but the UI needs to clearly support adding, listing, and managing multiple domains per app.

## Acceptance Criteria

- [ ] Domains tab on app detail shows all domains as a list
- [ ] "Add domain" button allows adding additional domains (not replacing the existing one)
- [ ] Each domain has its own SSL status, DNS verification status, and delete button
- [ ] Caddy routes generated for all domains pointing to the same container upstream
- [ ] Primary domain indicator — one domain marked as "primary" (used for preview env URLs, webhook displays)
- [ ] `PATCH /domains/{id}/primary` endpoint to set a domain as primary
- [ ] `primary` boolean column on the domains table (default false, exactly one true per app)

## Technical Notes

- The Caddy client already supports multiple routes — this is mostly UI work
- Ensure Caddy route generation iterates all domains for the app, not just the first

## Dependencies

- IF-023 (Domain management)
