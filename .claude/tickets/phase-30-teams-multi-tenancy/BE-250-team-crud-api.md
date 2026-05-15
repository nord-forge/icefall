# BE-250: Team CRUD API

**Phase:** 30
**Priority:** High
**Size:** M
**Dependencies:** BE-248

## Description

REST endpoints for team management.

## Endpoints

- `POST /teams` — create team (any authenticated user)
- `GET /teams` — list user's teams
- `GET /teams/{id}` — team detail (members, resource counts)
- `PUT /teams/{id}` — update team (name, settings) — owner/admin only
- `DELETE /teams/{id}` — delete team — owner only, must have no resources
- `POST /teams/{id}/switch` — switch active team (updates session)

## Acceptance Criteria

- Given any authenticated user, when they POST /teams, then a new team is created and they become owner
- Given a team owner, when they DELETE a team with apps, then 400 is returned with "Team still has resources"
- Given a user in multiple teams, when they POST /teams/{id}/switch, then subsequent requests are scoped to the new team

## Out of Scope

Billing per team, team plan enforcement
