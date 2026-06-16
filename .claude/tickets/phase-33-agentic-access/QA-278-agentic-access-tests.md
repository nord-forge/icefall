# QA-278: Agentic access security & integration tests

**Phase:** 33
**Priority:** Critical
**Size:** M
**Dependencies:** BE-271, BE-272, BE-273, BE-274, BE-275

## Description

This feature grants a non-human principal real power, so its restrictions must be
**verified, not assumed**. Cover the security-critical invariants with tests:
agents can't log in, MCP-only tokens can't touch REST, capability gating holds
per tool, the global toggle and per-agent disable are hard kill switches, and
every agent action is audited.

## Changes / Test Matrix

- **Principal (BE-271):** agent user cannot `POST /auth/login`; cannot enroll
  2FA; cannot change password; migration preserves existing users and defaults
  `is_service_account = false`.
- **Scope isolation (BE-272):** MCP-only token → MCP allowed, all REST routes
  `403 insufficient_scope`; REST-only token → MCP `403`; legacy null-ability
  token → unchanged full access.
- **Per-tool gating (BE-273):** `mcp:read` token can read but not deploy/write;
  `mcp:read+deploy` can deploy but not write; the mapping-coverage test fails if
  any registered tool lacks a group.
- **Management & kill switches (BE-274):** create returns token once and never
  again; rotate invalidates the old token; disable blocks the next call
  immediately; global toggle off blocks all agent calls and refuses creation;
  non-admin is `403` on agent writes; REST abilities rejected in capabilities.
- **Audit (BE-275):** an agent tool call produces an `mcp.tool_call` entry with
  no secret values; lifecycle events record the acting admin.

## Acceptance Criteria

- All matrix rows above have passing tests.
- The mapping-coverage assertion is wired so adding an unmapped MCP tool turns
  the suite red.
- Tests run in the existing Rust test harness (and CI), against SQLite, with no
  external services.
- `cargo test`, `cargo fmt --check`, and `clippy` are clean.

## Out of Scope

- Load/perf testing of the MCP endpoint.
- Frontend component tests beyond what the project already does (FE-276 carries
  its own a11y verification).

## Security Notes

Treat the scope-isolation and kill-switch tests as the acceptance gate for the
whole phase: if an MCP-only token can reach a REST mutation, or a disabled agent
can still act, the feature is not shippable.
