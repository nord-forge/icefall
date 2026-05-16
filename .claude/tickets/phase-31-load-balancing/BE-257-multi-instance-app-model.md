# BE-257: Multi-instance app model

**Phase:** 31
**Priority:** Critical
**Size:** M
**Dependencies:** None

## Description

Extend the data model so an app can be deployed to multiple servers simultaneously.

## Changes

- New table `app_instances`: id, app_id, server_id, status (running/stopped/deploying/failed), container_id, host_port, created_at, updated_at
- Keep `app.server_id` as "primary server" for backward compatibility
- Add `app.desired_instances` (integer, default 1) — how many instances the user wants
- Add `app.lb_policy` (string, default "round_robin") — round_robin, least_conn, ip_hash, random

## Migration

- For each existing app with a running container, create one `app_instances` row

## Acceptance Criteria

- Given an app with server_id set, when the migration runs, then one app_instance row exists
- Given an app with desired_instances = 3, when queried, then the model reports the target count
- Given the app_instances table, when an instance fails, then its status can be updated independently

## Out of Scope

Auto-scaling, resource-based placement
