# BE-272: `mcp:*` ability family and MCP-only token enforcement

**Phase:** 33
**Priority:** Critical
**Size:** M
**Dependencies:** BE-271

## Description

Add an `mcp` ability family to the existing token-ability system
(`src/api/abilities.rs`, IF-168) so an agent's API token can be scoped to the
MCP surface **and nothing else**. An MCP-only token authenticates to the MCP
endpoint but is rejected on every direct REST route — this is the technical
mechanism that keeps "the human in control": the agent can only do what its MCP
scopes allow, never reach the raw API.

## Changes

- Extend `ALL_ABILITIES` with a grouped `mcp:` namespace that mirrors the MCP
  tool groups (see BE-273): `mcp:read`, `mcp:deploy`, `mcp:write`, `mcp:admin`.
  These are distinct from the existing REST abilities (`apps:read`, etc.).
- MCP endpoint auth (`src/api/routes/mcp.rs`): require at least one `mcp:*`
  ability on the caller's token; a token with only REST abilities cannot call
  MCP, and vice-versa.
- REST middleware (`src/api/middleware.rs`): if a token's abilities contain only
  `mcp:*` entries (no REST abilities), reject direct REST routes with
  `403 insufficient_scope`. The MCP endpoint itself is exempt from this check.
- Backward compatibility: tokens with `abilities = NULL` (full access, existing
  behavior) are unchanged. The MCP-only restriction applies only to tokens that
  carry `mcp:*` and no REST abilities — i.e. agent tokens.
- Document the precedence clearly in `abilities.rs`: a token is "MCP-only" iff
  every granted ability is in the `mcp:` namespace.

## Acceptance Criteria

- Given a token scoped `["mcp:read"]`, when it calls the MCP endpoint, then it is
  authenticated; when it calls any `GET /api/v1/apps`, then `403
  insufficient_scope`.
- Given a token scoped `["apps:read"]` (no `mcp:*`), when it calls the MCP
  endpoint, then `403 insufficient_scope`.
- Given a legacy token with `abilities = NULL`, when it calls MCP or REST, then
  behavior is unchanged (full access).
- The `mcp:*` abilities appear in the ability list returned to the UI so the
  capability picker (FE-276) can render them.

## Out of Scope

- Mapping individual MCP tools to the four groups — BE-273.
- The agent CRUD that issues these tokens — BE-274.

## Security Notes

The MCP-only check is deny-by-default for agent tokens: anything not explicitly
in the `mcp:` namespace is unreachable. Adding a *new* REST route later cannot
accidentally widen an agent's reach, because the agent holds no REST abilities.
