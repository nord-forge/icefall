# IF-241: Noise-free logs UI

**Phase:** 29 — Frontend UI
**Priority:** Low
**Estimate:** M

## Description

Build the smart log filtering view (IF-193).

## Acceptance Criteria

- [ ] Log viewer toggle: "Smart" / "Raw" view
- [ ] Smart view: collapse repetitive lines with count badge
- [ ] Anomaly highlighting (first-seen patterns get yellow highlight)
- [ ] Noise suppression (health check pings, static assets dimmed)
- [ ] Error clustering (stack traces grouped)
- [ ] Time gap markers between activity bursts
- [ ] Per-app custom noise/highlight pattern config in settings
- [ ] a11y: collapsed groups expandable, filter state announced

## Dependencies

- IF-193 (Noise-free logs backend)
