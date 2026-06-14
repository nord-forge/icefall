# BE-248: Team database schema and model

**Phase:** 30
**Priority:** Critical
**Size:** L
**Dependencies:** None

## Description

Create the `teams` table, `team_memberships` table, and add `team_id` foreign keys to all resource tables.

## Schema

- `teams`: id, name, slug, owner_id, plan (free/pro/enterprise), settings (JSON), created_at, updated_at
- `team_memberships`: id, team_id, user_id, role (owner/admin/member/viewer), invited_by, accepted_at, created_at
- `team_invitations`: id, team_id, email, role, token, expires_at, created_at

## Migrations

- Add `team_id` column (nullable initially) to: apps, databases, projects, servers, api_tokens, notification_channels
- Create default team for existing data (owned by first admin user)
- Backfill all existing resources with default team_id

## Acceptance Criteria

- Given a fresh install, when the first user registers, then a default team is created and the user is its owner
- Given existing data, when the migration runs, then all resources are assigned to the default team
- Given the teams table, when queried, then team_memberships correctly links users to teams

## Out of Scope

UI, API endpoints, permission enforcement
