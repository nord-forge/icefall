# BE-255: Cross-team resource sharing

**Phase:** 30
**Priority:** Low
**Size:** M
**Dependencies:** BE-249

## Description

Allow specific resources (servers) to be shared across teams. Servers are the primary sharing target — a single physical server may host apps from multiple teams.

## Changes

- `server_team_access` junction table: server_id, team_id, access_level (deploy/read-only)
- Control-plane server is implicitly shared with all teams
- Worker servers can be explicitly shared by their owning team
- Shared servers appear in the target team's server list with a "shared" badge

## Acceptance Criteria

- Given team X owns server S, when team X shares it with team Y as "deploy", then team Y can deploy apps to server S
- Given a shared server, when team Y lists servers, then server S appears with a "shared" badge
- Given a shared server, when team Y tries to delete it, then 403 is returned

## Out of Scope

Shared apps, shared databases, resource quotas per team
