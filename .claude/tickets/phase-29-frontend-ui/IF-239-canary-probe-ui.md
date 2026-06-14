# IF-239: Canary probe results UI

**Phase:** 29 — Frontend UI
**Priority:** Low
**Estimate:** S

## Description

Surface canary probe results (IF-186) in the deploy detail.

## Acceptance Criteria

- [ ] App settings: "Enable Canary Probe" toggle + config (URL, request count, thresholds)
- [ ] Deploy detail: Canary Results section with latency percentiles, error rate, verdict badge
- [ ] Pass/fail indicator with comparison to baseline
- [ ] a11y: results table accessible

## Dependencies

- IF-186 (Canary probe backend)
