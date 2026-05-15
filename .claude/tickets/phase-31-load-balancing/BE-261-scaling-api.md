# BE-261: Scaling API

**Phase:** 31
**Priority:** High
**Size:** S
**Dependencies:** BE-258

## Description

API endpoints for managing app instance count and load balancing configuration.

## Endpoints

- `PUT /apps/{id}/scale` — `{ desired_instances: number }` — triggers deploy to reach desired count
- `GET /apps/{id}/instances` — list all instances with server, status, port, health
- `PUT /apps/{id}/lb-config` — `{ policy: string, health_check_path: string, sticky_sessions: boolean }`
- `DELETE /apps/{id}/instances/{instance_id}` — remove a specific instance

## Scaling Logic

- If desired > current: deploy new instances on servers with most capacity (use forecast data)
- If desired < current: stop excess instances, prefer stopping unhealthy ones first
- If desired = 0: stop all instances (equivalent to app stop)

## Acceptance Criteria

- Given an app with 1 instance, when PUT /scale with desired_instances = 3, then 2 new instances are started on available servers
- Given an app with 3 instances, when PUT /scale with desired_instances = 1, then 2 instances are stopped
- Given GET /instances, when called, then all instances with their server assignment and health status are returned

## Out of Scope

Auto-scaling rules, scheduled scaling
