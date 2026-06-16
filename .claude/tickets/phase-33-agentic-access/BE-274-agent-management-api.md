# BE-274: Agent management API and global toggle

**Phase:** 33
**Priority:** High
**Size:** M
**Dependencies:** BE-271, BE-272, BE-273

## Description

The admin-facing API to create and control agents. An admin enables agentic
access globally (default **off**), creates a named agent, picks its MCP
capabilities, and receives a scoped token **once**. They can rotate the token,
disable/enable the agent, and delete it — instant, total revocation. This is the
control surface that keeps the human in charge.

## Changes

- Global setting `agentic_access_enabled` (singleton settings row, default
  `false`, like `oauth_settings`). When false, the MCP endpoint rejects all
  agent-token calls (human tokens/sessions unaffected), and agent-create is
  refused.
- Endpoints (admin-only; enforced by existing role guard):
  - `GET /settings/agentic` — toggle state + list of agents (no secrets).
  - `PUT /settings/agentic` — enable/disable the global toggle.
  - `POST /agents` — create agent: `{ name, capabilities: ["mcp:read", ...] }`.
    Creates the `agent`-role service-account user (BE-271) + a scoped API token
    whose abilities are exactly the chosen `mcp:*` set. Returns the **plaintext
    token once**; only the SHA-256 hash is stored.
  - `GET /agents` / `GET /agents/{id}` — metadata (name, capabilities,
    created_at, last_used_at, enabled). Never returns the token.
  - `PUT /agents/{id}` — rename / change capabilities (re-scopes the token's
    abilities) / enable-disable.
  - `POST /agents/{id}/rotate-token` — revoke old token, issue + return a new one
    once.
  - `DELETE /agents/{id}` — delete the agent user + its token(s).
- Validation: capabilities must be a non-empty subset of the `mcp:*` family
  (reject any REST ability or unknown value). Default selection is `["mcp:read"]`
  if none supplied. Granting `mcp:write`/`mcp:deploy`/`mcp:admin` is allowed only
  by explicit inclusion (write opt-in).
- Disabling an agent must take effect immediately on the next request (no cached
  allow).

## Acceptance Criteria

- Given the global toggle is off, when an admin calls `POST /agents`, then it is
  refused with a clear message; when an existing agent token calls MCP, then it
  is rejected.
- Given an admin creates an agent with `["mcp:read","mcp:deploy"]`, then a
  plaintext token is returned exactly once and never retrievable again, and the
  token's abilities equal that set.
- Given an admin disables an agent, when that agent's token next calls MCP, then
  it is rejected without delay.
- Given `POST /agents/{id}/rotate-token`, then the previous token stops working
  and a new token is returned once.
- Given a non-admin caller, when hitting any `/agents` write endpoint, then
  `403`.
- Capabilities containing a REST ability (e.g. `apps:write`) are rejected.

## Out of Scope

- The dashboard UI — FE-276.
- Audit log writes — BE-275 (this ticket exposes the data; BE-275 records the
  actions).
- SSH-key issuance for agents — deferred (token is the credential).

## Security Notes

Token shown once, stored hashed — same posture as human API tokens. Deletion and
disable are the kill switches; both must be immediate and complete. The global
toggle is a master off-switch independent of individual agents.
