# IF-175: Deploy analytics dashboard

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Build a deploy analytics page that visualizes deployment frequency, success rate, average build time, and rollback rate over time. No other PaaS provides this level of deployment intelligence. Helps teams understand their deployment health and identify patterns.

## Acceptance Criteria

- [ ] New "Analytics" page accessible from the sidebar (admin/deployer)
- [ ] Time range selector: 7 days, 30 days, 90 days
- [ ] Metrics cards:
  - Total deploys in period
  - Success rate (%)
  - Average build time
  - Average deploy-to-live time (trigger → container swap)
  - Rollback rate (%)
- [ ] Charts:
  - Deploy frequency: bar chart (deploys per day/week)
  - Build time trend: line chart (average build time over time)
  - Success/failure ratio: stacked bar chart
  - Deploy heatmap: day-of-week × hour-of-day grid showing deploy density
- [ ] Per-app breakdown: table with app name, deploy count, success rate, avg build time, last deploy
- [ ] Filterable by app, server, and deploy trigger (manual/webhook/rollback)
- [ ] Data sourced from existing `deploys` table — no new data collection needed

## Technical Notes

- All data exists in the `deploys` table: `status`, `created_at`, `completed_at`, `trigger_type`
- Build time = `completed_at - created_at` for successful deploys
- Use a lightweight charting library (uPlot or Chart.js via Preact wrapper)
- Aggregate queries should be efficient on SQLite with proper indexes on `created_at` and `app_id`

## Dependencies

- IF-011 (Deploy records)
