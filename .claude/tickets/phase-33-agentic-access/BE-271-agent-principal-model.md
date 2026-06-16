# BE-271: Agent principal — `agent` role and service-account flag

**Phase:** 33
**Priority:** Critical
**Size:** M
**Dependencies:** None

## Description

Introduce a first-class non-human principal: the **AI agent**. An agent is a
`users` row with a new `agent` role and an `is_service_account` flag set true.
It exists so an MCP client (Claude, etc.) can act in Icefall under an identity
the human admin creates, scopes, audits, and revokes — never with a human's own
credentials.

The defining constraint: an agent can **never authenticate interactively**. It
has no usable password, cannot start a session via `POST /auth/login`, cannot
enroll 2FA, and cannot be promoted to a human role. Its only credential is a
scoped API token (BE-272).

## Changes

- Migration: extend the `users.role` CHECK constraint to include `'agent'`
  (`CHECK (role IN ('admin','deployer','viewer','agent'))`), and add
  `is_service_account BOOLEAN NOT NULL DEFAULT FALSE` to `users`.
  - SQLite cannot alter a CHECK constraint in place — recreate the table
    (create new → copy → drop → rename) inside the migration, preserving all
    existing columns/indexes/FKs.
- `User` model (`src/db/models/users.rs`): add `is_service_account: bool`;
  document that `role = "agent"` implies `is_service_account = true`.
- Auth guards (`src/api/routes/auth.rs`):
  - `POST /auth/login` rejects any user where `is_service_account` is true with
    a generic auth error (no principal-type leak).
  - 2FA enroll / password-change endpoints reject service accounts.
- A `password_hash` is not required for agents — store a non-loginable sentinel
  (e.g. `"!"`) so the Argon2 verify path can never succeed.
- Helper `User::is_agent(&self) -> bool` used by downstream tickets.

## Acceptance Criteria

- Given an agent user, when it attempts `POST /auth/login` with any input, then
  the response is the generic invalid-credentials error and no session is
  created.
- Given an agent user, when 2FA-enroll or password-change is called for it, then
  the request is rejected.
- A migration applied to an existing DB preserves all current users and their
  roles, and `is_service_account` defaults to `false` for them.
- The role CHECK constraint accepts `'agent'` and still rejects unknown roles.
- Creating a normal human user is unaffected.

## Out of Scope

- The scoped token and `mcp:*` abilities — BE-272.
- Per-tool MCP gating — BE-273.
- The management API and UI — BE-274 / FE-276.

## Security Notes

The agent role is a *reduction* of privilege, never an escalation. An agent must
not be assignable team-owner/admin. The human who creates it stays in control:
they choose its capabilities (BE-273/274) and can disable it instantly (BE-274).
