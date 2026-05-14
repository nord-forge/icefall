# IF-189: Dead App Detector

**Phase:** 26 — Icefall+
**Priority:** Medium
**Estimate:** S

## Description

Flag apps that have received zero inbound requests for a configurable period, consumed no CPU beyond idle, or haven't been deployed in months. Suggests hibernation (Ghost Mode), deletion, or archival.

## Acceptance Criteria

- [ ] Background task: weekly scan of all apps for inactivity signals
- [ ] Inactivity criteria (any of):
  - No inbound HTTP requests in X days (default: 30)
  - CPU usage <1% average over last 7 days
  - No deploy in X days (default: 90)
  - No env var or setting change in X days (default: 90)
- [ ] Dashboard: "Inactive Apps" badge in sidebar with count
- [ ] Inactive apps list: app name, last activity type, last activity date, resource usage
- [ ] Actions per inactive app: "Hibernate" (Ghost Mode), "Delete", "Keep" (snooze for 30 days)
- [ ] Notification: `system.inactive_apps` weekly digest email listing inactive apps
- [ ] Per-app setting: "Exempt from inactivity detection" toggle
- [ ] Request tracking: lightweight counter per app (increment on Caddy access log, not per-request DB write)

## Technical Notes

- Request counting: parse Caddy access logs or use Caddy's `metrics` module to track per-upstream request counts
- The scan is a simple SQL query: join apps with deploys (last deploy date) and metrics (average CPU)
- Store last-request timestamp in memory (agent-side), flush to SQLite hourly

## Dependencies

- IF-183 (Ghost Mode — for hibernate action)
- IF-026 (Container metrics — CPU usage data)
