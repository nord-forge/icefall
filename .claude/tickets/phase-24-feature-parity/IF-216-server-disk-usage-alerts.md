# IF-216: Server disk usage threshold alerts

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

IF-167 covers wiring server-reachable/unreachable and backup notifications. This ticket adds disk usage threshold alerts: notify users when a server's disk crosses a configurable warning threshold (e.g., 80%) and a critical threshold (e.g., 90%). Prevents the "server ran out of disk and everything crashed" scenario.

## Acceptance Criteria

- [ ] Per-server fields: `disk_alert_warning_threshold` (%, default 80), `disk_alert_critical_threshold` (%, default 90), `disk_alert_enabled` (bool, default true)
- [ ] Server settings page: disk alert configuration section
- [ ] Background check runs on the metrics polling interval (already 10s via IF-127)
- [ ] When disk usage crosses warning threshold: dispatch `server.disk.warning` notification (once, not repeated until it drops below and crosses again)
- [ ] When disk usage crosses critical threshold: dispatch `server.disk.critical` notification
- [ ] When disk usage drops back below warning: dispatch `server.disk.recovered` notification
- [ ] Notification payload includes: server name, current usage %, mount point, available space
- [ ] Server overview page: visual disk indicator turns amber at warning, red at critical
- [ ] Event types added to the notification subscription matrix (IF-071)
- [ ] API: thresholds configurable via `PUT /servers/{id}`

## Technical Notes

- Use hysteresis to prevent alert flapping: only re-alert if usage drops 5% below threshold before next alert
- Metrics data already collected by IF-127 (agent metrics) — this just adds threshold evaluation and notification dispatch
- Check all mount points, alert on the first one that crosses (typically `/` or the container runtime data directory)

## Out of Scope

- Automatic remediation (auto-trigger container cleanup)
- Per-mount-point threshold configuration
- Historical disk usage trend alerts ("disk full in N days" — that's IF-176)

## Dependencies

- IF-127 (Agent metrics collection — provides disk usage data)
- IF-043 (Notification system — dispatch mechanism)
- IF-071 (Notification subscriptions — event matrix)
