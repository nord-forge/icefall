# BE-259: Caddy multi-upstream routing

**Phase:** 31
**Priority:** Critical
**Size:** M
**Dependencies:** BE-258

## Description

Update Caddy route management to configure multiple upstreams per domain with load balancing policy and health checks.

## Changes to `src/caddy/routes.rs`

- New method: `add_route_balanced(domain: &str, upstreams: &[String], policy: &str)`
- Caddy config generates `reverse_proxy` with multiple `upstreams` entries
- Add `lb_policy` mapping: round_robin, least_conn, ip_hash, random -> Caddy policy names
- Enable passive health checks: `fail_duration 30s`, `max_fails 3`, `unhealthy_latency 5s`
- Enable active health checks: `interval 10s`, `path /` (configurable per app)
- When an upstream fails health check, Caddy automatically removes it from rotation

## Caddy Config Structure

```json
{
  "handle": [{
    "handler": "reverse_proxy",
    "upstreams": [
      {"dial": "server1:8001"},
      {"dial": "server2:8001"}
    ],
    "load_balancing": {
      "selection_policy": {"policy": "round_robin"}
    },
    "health_checks": {
      "passive": {"fail_duration": "30s", "max_fails": 3},
      "active": {"interval": "10s", "path": "/health"}
    }
  }]
}
```

## Acceptance Criteria

- Given 2 upstreams, when Caddy config is generated, then both upstreams are listed with the configured policy
- Given an upstream that fails 3 requests, when Caddy health check runs, then traffic stops going to that upstream
- Given a healthy upstream recovers, when Caddy health check passes, then traffic resumes

## Out of Scope

Weighted routing, canary percentage splits, geographic routing
