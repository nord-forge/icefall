# IF-232: Incident timeline UI

**Phase:** 29 — Frontend UI
**Priority:** Medium
**Estimate:** M

## Description

Build the incidents page (IF-178). Chronological incident list with status management and note timeline.

## Acceptance Criteria

- [ ] New sidebar nav item: "Incidents"
- [ ] Incident list: title, status badge, severity, affected apps, duration, created time
- [ ] Create incident form: title, severity selector, affected apps multi-select
- [ ] Incident detail: status update buttons (investigating → identified → monitoring → resolved)
- [ ] Notes timeline: chronological note list with author and timestamp
- [ ] Add note form at bottom of timeline
- [ ] Status page toggle per app (IF-178 public status page)
- [ ] a11y: status badges have text labels, timeline navigable by keyboard

## Dependencies

- IF-178 (Incident timeline backend)
