# IF-231: Deploy analytics dashboard UI

**Phase:** 29 — Frontend UI
**Priority:** Medium
**Estimate:** M

## Description

Build the analytics page (IF-175). Visualize deploy frequency, success rate, build times with charts.

## Acceptance Criteria

- [ ] New sidebar nav item: "Analytics"
- [ ] Time range selector: 7d / 30d / 90d
- [ ] Metrics cards: total deploys, success rate, avg build time, rollback rate
- [ ] Charts (use lightweight library — uPlot or Chart.js):
  - Deploy frequency bar chart
  - Build time trend line chart
  - Success/failure stacked bar
- [ ] Per-app breakdown table
- [ ] Filter by app, server, trigger type
- [ ] a11y: chart data also available as tables, color-blind safe palette

## Dependencies

- IF-175 (Deploy analytics backend)
