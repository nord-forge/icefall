# IF-176: Resource usage forecasting

**Phase:** 26 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Use historical metrics data to project when a server will run out of disk space, memory, or CPU headroom. Display a "days until full" estimate on the server detail page. No other PaaS does predictive resource planning — this helps solo devs avoid surprise outages.

## Acceptance Criteria

- [ ] Server detail page: "Forecast" section below current metrics
- [ ] Disk forecast: "At current growth rate, disk will be full in ~X days" with a trend line chart
- [ ] Memory forecast: average utilization trend, alert if trending above 90% over the next 7 days
- [ ] CPU forecast: average load trend
- [ ] Calculation: linear regression on the last 30 days of metrics data
- [ ] Visual: trend line extended into the future (dotted line) on the metrics chart
- [ ] Warning badge on server card when any resource is projected to exhaust within 14 days
- [ ] API endpoint: `GET /servers/{id}/forecast` returns projected exhaustion dates

## Technical Notes

- Metrics history is already stored in `server_metrics_history` (IF-026, IF-127)
- Linear regression on daily aggregates — use a simple least-squares fit, no ML library needed
- For disk: track `disk_used` growth rate. For memory: track `memory_used` peaks. For CPU: track `cpu_percent` average.
- Only show forecast when there's at least 7 days of data

## Dependencies

- IF-026 (Container metrics collection)
- IF-127 (Agent metrics collection)
- IF-138 (Server detail page)
