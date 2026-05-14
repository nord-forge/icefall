# IF-227: Log drains configuration UI

**Phase:** 29 — Frontend UI
**Priority:** Medium
**Estimate:** M

## Description

Build the dashboard UI for log drains (IF-150). Users need to configure Loki, Axiom, and generic HTTP drains per-app and globally.

## Acceptance Criteria

- [ ] App detail → Logs tab: "Log Drains" section below the log viewer
- [ ] Drain list: name, type icon (Loki/Axiom/HTTP), status badge (active/disabled/error), last sent
- [ ] Add drain form: type selector → type-specific config fields
  - Loki: URL, tenant ID, username, password, custom labels
  - Axiom: dataset name, API token
  - HTTP: URL, method, headers, format selector (JSON lines/array/text)
- [ ] "Test Connection" button per drain
- [ ] Error indicator with last error message
- [ ] Enable/disable toggle per drain
- [ ] Settings page: "Global Log Drains" section (admin only)
- [ ] Password/token fields use reveal toggle
- [ ] a11y: all inputs labeled, error states announced

## Dependencies

- IF-150 (Log drains backend)
