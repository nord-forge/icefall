# QA-264: Load balancing integration tests

**Phase:** 31
**Priority:** High
**Size:** M
**Dependencies:** BE-258, BE-259

## Description

End-to-end tests for multi-instance deployment and traffic distribution.

## Test Scenarios

- Deploy app with desired_instances = 2: both instances start, Caddy config has 2 upstreams
- Scale up from 1 to 3: 2 new instances deployed, Caddy updated
- Scale down from 3 to 1: 2 instances stopped, Caddy updated
- Instance failure: health monitor detects, instance replaced, Caddy updated
- Rolling deploy: instances updated one at a time, zero downtime verified
- LB policy change: Caddy config updated without restarting instances
- Sticky sessions: same client IP routes to same instance

## Acceptance Criteria

- All multi-instance scenarios pass
- Zero-downtime rolling deploy verified (no 502s during deploy)
- Caddy config always reflects actual running instances

## Out of Scope

Performance/load testing, cross-datacenter scenarios
