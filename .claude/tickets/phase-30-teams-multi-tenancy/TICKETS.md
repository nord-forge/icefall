# Phase 30: Teams & Multi-Tenancy

> Priority: v2.0
> Estimated effort: XL (6-8 weeks)
> Dependencies: None (multi-server already shipped in Phase 20)

## Overview

Add team-based resource ownership and multi-tenant isolation to Icefall. Currently all resources are global — any admin/deployer can see and manage everything. This phase introduces teams as the ownership boundary, scoped permissions, and the infrastructure to support multiple independent groups on a single Icefall instance.

## Current State

- **User model:** `id`, `email`, `password_hash`, `role` (admin/deployer/viewer), `totp_*`, timestamps
- **No team/org concept** anywhere in the codebase
- **No `team_id` or `org_id`** on any resource table (apps, databases, servers, projects, etc.)
- **Audit log** exists (`src/api/routes/audit.rs`) but is not team-scoped
- **API tokens** exist but are user-scoped, not team-scoped

## Tickets

| ID | Title | Priority | Size | Dependencies | Status |
|---|---|---|---|---|---|
| [BE-248](BE-248-team-database-schema.md) | Team database schema and model | Critical | L | None | Not started |
| [BE-249](BE-249-team-scoped-queries.md) | Team-scoped queries and middleware | Critical | XL | BE-248 | Not started |
| [BE-250](BE-250-team-crud-api.md) | Team CRUD API | High | M | BE-248 | Not started |
| [BE-251](BE-251-team-membership-invitation-api.md) | Team membership and invitation API | High | M | BE-250 | Not started |
| [BE-252](BE-252-team-scoped-api-tokens.md) | Team-scoped API tokens | Medium | S | BE-249 | Not started |
| [FE-253](FE-253-team-management-ui.md) | Team management UI | High | L | BE-250, BE-251 | Not started |
| [FE-254](FE-254-team-onboarding-flow.md) | Team onboarding flow | Medium | S | FE-253 | Not started |
| [BE-255](BE-255-cross-team-resource-sharing.md) | Cross-team resource sharing | Low | M | BE-249 | Not started |
| [QA-256](QA-256-team-isolation-tests.md) | Team isolation integration tests | High | M | BE-249, BE-251 | Not started |

## Dependency Graph

```
BE-248 (schema)
  ├── BE-249 (scoped queries)
  │     ├── BE-252 (scoped tokens)
  │     ├── BE-255 (cross-team sharing)
  │     └── QA-256 (isolation tests)
  └── BE-250 (CRUD API)
        └── BE-251 (membership API)
              ├── FE-253 (team UI)
              │     └── FE-254 (onboarding)
              └── QA-256 (isolation tests)
```
