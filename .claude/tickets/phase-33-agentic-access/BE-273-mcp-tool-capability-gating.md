# BE-273: Per-tool MCP capability gating

**Phase:** 33
**Priority:** Critical
**Size:** M
**Dependencies:** BE-272

## Description

Today the MCP server gates tools by `user.role` only (admin/deployer can write,
everyone can read — `src/api/routes/mcp.rs`). Replace that with **capability
gating**: every MCP tool is tagged with the `mcp:*` ability it requires, and a
caller may invoke a tool only if its token holds that ability. This is what lets
an admin grant an agent "read + deploy" but not "write/admin", per the locked
design (admin chooses, write opt-in, read-only default).

Crucially, **future MCP tools join a group**, so a tool added later is
automatically covered by whichever capability its group maps to — no agent
silently gains access to an ungated tool.

## Changes

- Define a single source of truth mapping each MCP tool name → required ability:
  - `mcp:read` — `list_apps`, `get_app`, `get_deploy_status`, `get_logs`,
    `get_env_vars`, `list_databases`, `get_health_status`, `get_server_status`,
    `diagnose`, `suggest_fix`, `list_servers`, `server_forecast`,
    `export_bundle`, `search`, `get_analytics`, MCP resources, MCP prompts.
  - `mcp:deploy` — `deploy_app`, `cancel_deploy`, `rollback_deploy`,
    `approve_deploy`, `bulk_deploy`, `bulk_restart`, `restart_app`,
    `deploy_workflow`, `rollback_if_unhealthy`.
  - `mcp:write` — `set_env_var`, `bulk_env_set`, `create_database`, `add_domain`,
    `create_app`, `import_bundle`.
  - `mcp:admin` — reserved for future destructive/global tools (e.g.
    `server_optimize`, delete operations). Empty set is acceptable at ship.
- `call_tool` (and the resources/prompts handlers) look up the tool's required
  ability and check it against the caller's token abilities before dispatch;
  reject with `403 insufficient_scope` (tool name in `details`, not the message).
- A compile-time or test-time assertion that **every** registered tool has a
  group mapping — a new tool with no mapping fails the test (the auto-coverage
  guarantee).
- Human callers (session / full-access token, `abilities = NULL`) keep their
  existing role-based behavior — capability gating applies to scoped tokens.

## Acceptance Criteria

- Given an agent token scoped `["mcp:read"]`, when it calls `deploy_app`, then
  `403 insufficient_scope`; when it calls `get_logs`, then success.
- Given an agent token scoped `["mcp:read","mcp:deploy"]`, when it calls
  `deploy_app`, then success; when it calls `set_env_var`, then `403`.
- Given a newly added MCP tool with no group mapping, when the mapping-coverage
  test runs, then it fails.
- Given a human admin session, when it calls any tool, then behavior is unchanged
  from today.

## Out of Scope

- The ability namespace itself and REST blocking — BE-272.
- Recording the call in the audit log — BE-275.

## Security Notes

Default-deny per tool: a scoped agent token can invoke only the explicit
intersection of (tools in its granted groups). The mapping-coverage test is the
safety net that prevents a future tool from being reachable without a deliberate
group assignment.
