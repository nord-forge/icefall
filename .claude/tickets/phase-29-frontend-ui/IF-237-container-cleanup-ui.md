# IF-237: Container cleanup settings UI

**Phase:** 29 — Frontend UI
**Priority:** Low
**Estimate:** S

## Description

Build the cleanup configuration UI (IF-152 + IF-214).

## Acceptance Criteria

- [ ] Settings page: "Container Cleanup" section
- [ ] Schedule config: cron expression or preset (daily/weekly)
- [ ] Disk threshold slider (50%-95%)
- [ ] Toggles: dangling images, unused images, stopped containers, volumes, networks
- [ ] "Run Now" button
- [ ] Cleanup history: last 10 runs with space reclaimed
- [ ] Per-server cleanup config on server detail
- [ ] a11y: slider accessible, toggle labels clear

## Dependencies

- IF-152, IF-214 (Cleanup backend)
