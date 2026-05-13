# IF-179: Scheduled deploys

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Allow users to schedule a deploy for a specific time instead of triggering it immediately. Useful for coordinating releases during maintenance windows, deploying during low-traffic hours, or scheduling deployments across time zones. No other self-hosted PaaS has this.

## Acceptance Criteria

- [ ] Deploy dialog: "Deploy now" (default) or "Schedule for later" option
- [ ] Date/time picker for scheduled time (respects user's timezone preference from IF-084)
- [ ] Scheduled deploys appear in the deploy history with a "scheduled" status and countdown
- [ ] Background scheduler checks for due deploys every 30 seconds
- [ ] When the scheduled time arrives: trigger the deploy automatically
- [ ] Cancel button: cancel a scheduled deploy before it triggers
- [ ] Reschedule: change the scheduled time
- [ ] If the server is offline when the deploy is due: retry for up to 30 minutes, then mark as "missed"
- [ ] Notification: dispatch `deploy.scheduled` event when scheduling and `deploy.started` when it triggers
- [ ] `scheduled_at` nullable timestamp column on the `deploys` table
- [ ] API: `POST /apps/{id}/deploy` accepts optional `scheduled_at` ISO 8601 timestamp
- [ ] Calendar view: optional month view showing scheduled and past deploys (stretch goal)

## Technical Notes

- Reuse the existing deploy pipeline — scheduled deploys just delay the trigger
- The scheduler is a simple `tokio::spawn` loop checking `WHERE scheduled_at <= NOW() AND status = 'scheduled'`
- For timezone handling: store all times in UTC, display in the user's configured timezone

## Dependencies

- IF-011 (Container deployment)
- IF-084 (User preferences — timezone)
