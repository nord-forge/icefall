# IF-226: Shared variables management UI

**Phase:** 29 — Frontend UI
**Priority:** High
**Estimate:** M

## Description

Build the dashboard UI for the shared variables system (IF-209). Users need a sidebar nav item "Shared Variables" with scope tabs (Project / Server), variable CRUD, and a resolved-variables view on app detail.

## Acceptance Criteria

- [ ] New sidebar nav item: "Shared Variables"
- [ ] Page with tabs: Project / Server scope selector
- [ ] Per-scope: list of key-value pairs with add/edit/delete
- [ ] Values masked by default (click to reveal for sensitive vars)
- [ ] Import from `.env` file (paste or upload)
- [ ] Developer view toggle: raw `.env` textarea
- [ ] App detail → Env tab: "Inherited Variables" section showing resolved cascade with source badges ("from project: X", "from server: Y")
- [ ] All forms follow existing EnvVarEditor patterns
- [ ] a11y: form labels, focus management, keyboard navigation

## Dependencies

- IF-209 (Shared variables backend)
