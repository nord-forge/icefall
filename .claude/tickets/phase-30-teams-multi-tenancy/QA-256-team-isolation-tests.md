# QA-256: Team isolation integration tests

**Phase:** 30
**Priority:** High
**Size:** M
**Dependencies:** BE-249, BE-251

## Description

End-to-end tests verifying team isolation guarantees.

## Test Scenarios

- User in team A cannot read team B's apps, databases, servers, projects, env vars, domains
- User in team A cannot modify team B's resources even by guessing IDs
- API token scoped to team A cannot access team B
- Team deletion blocked when resources exist
- Member removal revokes all access immediately
- Cross-team server sharing works correctly
- Switching teams changes all query scopes

## Acceptance Criteria

- All isolation scenarios pass with zero cross-team data leakage
- No 500 errors — all unauthorized access returns 404 (prevents existence enumeration)

## Out of Scope

Performance testing, load testing multi-tenant scenarios
