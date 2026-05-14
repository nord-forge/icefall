# IF-229: Project environments management UI

**Phase:** 29 — Frontend UI
**Priority:** High
**Estimate:** L

## Description

Build the UI for project environments (IF-147). Projects get environment tabs (Production/Staging/Development), apps are assigned to environments, and environment-scoped variables cascade.

## Acceptance Criteria

- [ ] Project detail page: environment tab bar (colored pills)
- [ ] Each environment tab lists its apps with status indicators
- [ ] Environment variables editor (same UX as app env var editor)
- [ ] "Resolved Variables" read-only view on app detail showing merged set with source badges
- [ ] App settings: environment assignment dropdown
- [ ] Create environment form: name, color picker, sort order
- [ ] Delete environment (with confirmation + reassignment option)
- [ ] Environment badge on AppCard in project view
- [ ] Empty state: "Move or create an app in this environment"
- [ ] a11y: tab panels, color contrast for badges, keyboard navigation

## Dependencies

- IF-147 (Environments backend)
