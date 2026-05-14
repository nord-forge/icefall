# IF-194: Power Nap Scheduler

**Phase:** 26 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Define quiet hours when low-priority apps are automatically suspended and high-priority apps have their resource limits reduced. Apps wake on schedule or on first request. Meaningful cost savings for VPS users who don't need all containers running 24/7.

## Acceptance Criteria

- [ ] Per-app setting: priority level (critical / standard / low)
  - **Critical**: never suspended, full resources always
  - **Standard**: reduced resources during quiet hours (50% CPU/memory limits)
  - **Low**: suspended during quiet hours, wake on request or schedule
- [ ] Global setting: quiet hours schedule (e.g., "01:00-07:00 server time" or cron expression)
- [ ] Per-app override: custom quiet hours per app
- [ ] At quiet hour start: suspend "low" apps, reduce "standard" app resources
- [ ] At quiet hour end: wake all suspended apps, restore resource limits
- [ ] Wake-on-request: if a request hits a suspended app during quiet hours, wake it (via Ghost Mode infrastructure)
- [ ] Dashboard: schedule visualization showing which apps are active/suspended over a 24-hour period
- [ ] Savings report: "Power Nap saved ~X MB RAM during quiet hours last week"
- [ ] Notification: `system.power_nap_start` and `system.power_nap_end` events

## Technical Notes

- Builds on Ghost Mode (IF-183) for the suspend/wake infrastructure
- Resource reduction: update container resource limits via the container runtime API (Docker `docker update` / Podman `podman update` — no restart needed for memory/CPU limits)
- Schedule stored in settings, evaluated by the background scheduler
- For multi-server: each server runs its own Power Nap schedule independently

## Dependencies

- IF-183 (Ghost Mode — suspend/wake mechanism)
- IF-061 (Resource limits — for reduction during quiet hours)
