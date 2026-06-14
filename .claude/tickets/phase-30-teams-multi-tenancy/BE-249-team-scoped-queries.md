# BE-249: Team-scoped queries and middleware

**Phase:** 30
**Priority:** Critical
**Size:** XL
**Dependencies:** BE-248

## Description

Modify every database query to filter by the authenticated user's active team. Add middleware that resolves the current team from the session/token.

## Changes

- Add `active_team_id` to session data
- Create `TeamMiddleware` that resolves team from session and injects into request extensions
- Update all `list_*` queries: `WHERE team_id = ?`
- Update all `get_*` queries: validate resource belongs to user's team
- Update all `create_*` queries: set `team_id` from session
- Update all `update_*` / `delete_*` queries: validate team ownership before mutation

## Acceptance Criteria

- Given user A in team X and user B in team Y, when user A lists apps, then only team X apps are returned
- Given user A in team X, when user A tries to access a team Y resource, then 404 is returned (not 403, to avoid leaking existence)
- Given an API token, when used for requests, then the token's team_id scopes all queries

## Out of Scope

Cross-team resource sharing, team switching UI
