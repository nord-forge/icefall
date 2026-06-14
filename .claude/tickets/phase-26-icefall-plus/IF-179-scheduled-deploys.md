# IF-179: Scheduled deploys

**Phase:** 26 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Allow users to schedule a deploy for a specific time instead of triggering it immediately. Useful for coordinating releases during maintenance windows, deploying during low-traffic hours, or scheduling deployments across time zones. No other self-hosted PaaS has this.

## Acceptance Criteria

- [x] Deploy dialog: "Deploy now" (default) or "Schedule for later" option — AppHeader deploy menu → ScheduleDeployDialog
- [x] Date/time picker for scheduled time — `datetime-local` picker interpreted in the **user's configured timezone** (profile preference, IF-084; browser tz fallback), converted to UTC for storage via `@lib/timezone`
- [x] Scheduled deploys appear in the deploy history with a "scheduled" status and countdown — DeploysTab `scheduled` StatusDot + `formatCountdown`
- [x] Background scheduler checks for due deploys every 30 seconds — `deploy::scheduler::spawn_deploy_scheduler`
- [x] When the scheduled time arrives: trigger the deploy automatically — via shared `trigger_deploy`
- [x] Cancel button: cancel a scheduled deploy before it triggers — `cancel_deploy` now accepts `scheduled`
- [x] Reschedule: change the scheduled time — `POST /deploys/{id}/reschedule` + dialog
- [x] If the server is offline when the deploy is due: retry for up to 30 minutes, then mark as "missed" — 30-min grace window in scheduler
- [x] Notification: dispatch `deploy.scheduled` event when scheduling and `deploy.started` when it triggers — plus `deploy.missed`
- [x] `scheduled_at` nullable timestamp column on the `deploys` table — already present; migration adds `scheduled`/`missed` statuses
- [x] API: `POST /apps/{id}/deploys` accepts optional `scheduled_at` ISO 8601 timestamp
- [ ] Calendar view: optional month view showing scheduled and past deploys (stretch goal) — **skipped** (explicit stretch goal)

## Technical Notes

- Reuse the existing deploy pipeline — scheduled deploys just delay the trigger
- The scheduler is a simple `tokio::spawn` loop checking `WHERE scheduled_at <= NOW() AND status = 'scheduled'`
- For timezone handling: store all times in UTC, display in the user's configured timezone

## Dependencies

- IF-011 (Container deployment)
- IF-084 (User preferences — timezone)
