# BE-260: Instance health monitoring

**Phase:** 31
**Priority:** High
**Size:** M
**Dependencies:** BE-257

## Description

Monitor health of individual app instances across servers. Replace/restart failed instances.

## Changes

- Extend `health_runner.rs` to check each instance independently
- Instance status transitions: deploying -> running -> unhealthy -> stopped
- After N consecutive health failures on an instance, mark as failed
- If desired_instances > running instances, attempt to start a replacement on the same or another server
- SSE events: `instance.healthy`, `instance.unhealthy`, `instance.replaced`

## Acceptance Criteria

- Given 3 instances with 1 failing health checks, when the failure threshold is reached, then only the failing instance is restarted
- Given a failed instance that cannot restart on its server, when another server has capacity, then the instance is placed there
- Given an instance status change, when the SSE stream is open, then the event is emitted

## Out of Scope

Auto-scaling based on load, predictive health
