# IF-167: Server, disk, and backup notification alerts

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** M

## Description

Wire up three missing notification event types that have supporting infrastructure but no dispatch: server reachability changes, disk usage threshold alerts, and backup outcome notifications. The notification system (IF-043) and metrics collection (IF-026, IF-127) are already built — this ticket connects them.

## Acceptance Criteria

### Server Reachability Alerts
- [ ] When a server's status changes to `offline` (heartbeat timeout): dispatch `server.offline` event
- [ ] When a server reconnects: dispatch `server.online` event
- [ ] Events include server name, ID, and duration offline
- [ ] Subscribable per-channel via the existing notification rules (IF-071)

### Disk Usage Alerts
- [ ] Configurable disk usage threshold in Settings (default: 80%)
- [ ] When server disk usage exceeds threshold: dispatch `system.disk_warning` event
- [ ] Alert includes: server name, current usage %, threshold %, available space
- [ ] Cooldown: don't re-alert for the same server within 1 hour
- [ ] Check runs every 10 minutes (piggyback on metrics collection cycle)
- [ ] Subscribable per-channel

### Backup Alerts
- [ ] On backup success: dispatch `backup.success` event (database name, size, duration)
- [ ] On backup failure: dispatch `backup.failure` event (database name, error message)
- [ ] Subscribable per-channel
- [ ] Add `backup.success` and `backup.failure` to the notification subscription matrix (IF-071)

### Dashboard
- [ ] New event types appear in the notification subscription checkboxes
- [ ] Settings page: disk usage threshold slider (50%-95%)

## Technical Notes

- Server status changes already trigger SSE events (IF-119) — hook into the same state change to dispatch notifications
- Disk usage is collected by `sysinfo` crate every 10 seconds — check threshold on each collection
- Backup scheduler (IF-030) already has success/failure paths — add notification dispatch calls

## Dependencies

- IF-043 (Notification system)
- IF-071 (Notification subscriptions UI)
- IF-026 (Container metrics)
- IF-119 (Agent WebSocket — server status)
- IF-030 (Database backups)
