# BE-252: Team-scoped API tokens

**Phase:** 30
**Priority:** Medium
**Size:** S
**Dependencies:** BE-249

## Description

API tokens are scoped to a team. When authenticating with a token, the team context is set automatically.

## Changes

- Add `team_id` to `api_tokens` table
- Token creation sets `team_id` from session's active team
- Token auth middleware sets team context from token's `team_id`
- Token list only shows tokens for current team

## Acceptance Criteria

- Given a token created in team X, when used for API calls, then all queries are scoped to team X
- Given a user in multiple teams, when they list tokens, then only current team's tokens are shown

## Out of Scope

Cross-team tokens, token permission scopes (read-only vs read-write)
