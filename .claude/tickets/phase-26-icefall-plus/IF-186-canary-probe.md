# IF-186: Canary Probe — automated post-deploy regression detection

**Phase:** 26 — Icefall+
**Priority:** High
**Estimate:** M

## Description

After each deploy, automatically send a burst of synthetic HTTP requests to the new version, compare response times and error rates against the previous version's baseline, and auto-rollback if the new version is measurably worse.

## Acceptance Criteria

- [ ] Per-app setting: "Enable Canary Probe" toggle with configuration:
  - Probe URL path (default: `/` or the health check path)
  - Request count (default: 20)
  - Max acceptable p95 latency increase (default: 50%)
  - Max acceptable error rate (default: 5%)
  - Timeout per request (default: 5 seconds)
- [ ] After deploy succeeds and health check passes: run the canary probe
- [ ] Probe sends requests to the new container directly (internal URL), not through the public domain
- [ ] Results compared against baseline (stored from previous successful deploy's canary run)
- [ ] If p95 latency increases beyond threshold OR error rate exceeds threshold: auto-rollback + notification
- [ ] Deploy log: canary probe results section with latency histogram and pass/fail verdict
- [ ] First deploy (no baseline): store results as baseline, always pass
- [ ] `canary_results` table: `deploy_id`, `p50_ms`, `p95_ms`, `p99_ms`, `error_count`, `total_requests`, `verdict`

## Technical Notes

- Use `reqwest` with connection pooling to send concurrent probe requests from the Rust binary
- Run probes from the same server as the container for accurate latency measurement
- For multi-server: the agent runs the probe on the worker where the container lives
- Latency comparison: use relative increase, not absolute threshold (apps have different baselines)

## Dependencies

- IF-011 (Container deployment — trigger after success)
- IF-066 (Container rollback — for auto-rollback)
