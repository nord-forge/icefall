# IF-244: Config time machine UI

**Phase:** 29 — Frontend UI
**Priority:** Medium
**Estimate:** S

## Description

Surface config history (IF-187) in the dashboard.

## Acceptance Criteria

- [ ] App settings: "History" button → timeline view of config changes
- [ ] Each entry: field changed, old → new value, who, when
- [ ] Env var values masked by default
- [ ] "Restore" button per entry
- [ ] Diff view: select two timestamps
- [ ] a11y: timeline navigable, restore action confirmed

## Dependencies

- IF-187 (Config time machine backend)
