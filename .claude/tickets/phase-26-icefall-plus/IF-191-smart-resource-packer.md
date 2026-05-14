# IF-191: Smart Resource Packer

**Phase:** 26 — Icefall+
**Priority:** High
**Estimate:** L

## Description

Analyze actual CPU/memory usage patterns across all containers and suggest right-sized resource limits, container co-location, and scheduling optimizations to minimize VPS cost. Shows concrete savings estimates.

## Acceptance Criteria

- [ ] "Optimization" section on server detail page
- [ ] Recommendations engine analyzes 7 days of metrics data:
  - **Over-provisioned**: containers with memory limit 2x+ above peak usage → suggest lower limit
  - **Under-provisioned**: containers hitting memory/CPU limits → suggest higher limit or alert
  - **Idle resources**: containers using <5% CPU consistently → suggest Ghost Mode
  - **Co-location**: if running multiple servers, suggest moving low-resource apps to fill underutilized servers
- [ ] Each recommendation shows: current config, suggested config, estimated savings (RAM freed, cost saved)
- [ ] "Apply" button: one-click to apply the suggested resource limits
- [ ] "Apply all" button: apply all non-destructive recommendations at once
- [ ] Weekly digest notification: "You could save ~X MB RAM by right-sizing 3 containers"
- [ ] API: `GET /servers/{id}/optimizations` returns structured recommendations

## Technical Notes

- Analysis runs on the control plane using data from `server_metrics_history` and `container_stats`
- Memory right-sizing: suggest limit at peak usage + 20% headroom
- CPU right-sizing: suggest shares proportional to average usage
- Cost estimation: map RAM/CPU to approximate VPS pricing tiers

## Dependencies

- IF-026 (Container metrics)
- IF-127 (Agent metrics)
- IF-176 (Resource forecasting — complementary feature)
