# IF-233: Scheduled tasks UI

**Phase:** 29 — Frontend UI
**Priority:** Medium
**Estimate:** S

## Description

Build the scheduled tasks tab on app detail (IF-208). Task CRUD, execution history, manual trigger.

## Acceptance Criteria

- [ ] App detail: new "Tasks" tab
- [ ] Task list: name, cron expression (human-readable preview), last run status, enabled toggle
- [ ] Create task form: name, command, cron expression input with preview, timeout, container selector
- [ ] "Run Now" button per task
- [ ] Execution history: expandable rows with output viewer
- [ ] a11y: toggle switches labeled, output viewer scrollable with keyboard

## Dependencies

- IF-208 (Scheduled tasks backend)
