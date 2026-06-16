# Phase 33: Agentic Access

> **Status: PLANNED**
> Priority: v2.0
> Estimated effort: L (1-2 weeks)
> Dependencies: builds on the user/auth model (Phase 8), API token ability
> scoping (IF-168), the MCP server (IF-044 / Phase 27), and audit logging
> (IF-145).

## Overview

First-party support for **agentic users**: a dedicated, non-human principal that
an MCP client (Claude, etc.) uses to act in Icefall under an identity the human
admin creates, scopes, audits, and can revoke at any time. The guiding principle
is **human-in-control, least-privilege**: an agent can only ever do what its
admin explicitly grants through the MCP capability set, and nothing else.

The agent is a service-account `user` with a new `agent` role that **cannot log
in interactively** (no password session, no 2FA). Its sole credential is a
scoped API token whose abilities live entirely in a new `mcp:*` namespace —
which both authorizes the MCP endpoint and **blocks all direct REST access**. So
the agent reaches the platform only through MCP tools, only for the capabilities
it was given. Adding a new MCP tool later auto-joins a capability group, and a
mapping-coverage test prevents any tool from being silently ungated.

Agentic access is **off by default**. An admin enables it globally, creates a
named agent, picks its capabilities (Read by default; Deploy/Write/Admin are
explicit opt-ins), and receives the token once. Disable or delete is an
immediate, total kill switch. Every agent action is written to the audit log.

## Design decisions (locked with the requester)

1. **Principal:** new `agent` role + `is_service_account` flag on `users` — a
   distinct principal that can never authenticate interactively.
2. **Auth + surface:** a scoped API token, **MCP-only** — a new `mcp:*` ability
   family gates the MCP endpoint and blocks direct REST.
3. **SSH:** deferred — the API token is the credential; SSH keys stay
   deploy-only. Agent SSH support can be added later if needed.
4. **Capability ceiling:** per-capability, admin-chosen, **write opt-in** —
   grouped Read / Deploy / Write / Admin, read-only default, future tools
   covered by their group automatically.

## Tickets

| ID | Title | Priority | Size | Dependencies | Status |
|---|---|---|---|---|---|
| [BE-271](BE-271-agent-principal-model.md) | Agent principal — `agent` role + service-account flag | Critical | M | None | Planned |
| [BE-272](BE-272-mcp-ability-scope.md) | `mcp:*` ability family + MCP-only token enforcement | Critical | M | BE-271 | Planned |
| [BE-273](BE-273-mcp-tool-capability-gating.md) | Per-tool MCP capability gating | Critical | M | BE-272 | Planned |
| [BE-274](BE-274-agent-management-api.md) | Agent management API + global toggle | High | M | BE-271, BE-272, BE-273 | Planned |
| [BE-275](BE-275-agent-audit-logging.md) | Audit logging for agent actions | High | S | BE-273, BE-274 | Planned |
| [FE-276](FE-276-agentic-access-settings-ui.md) | Agentic Access settings section | High | M | BE-274 | Planned |
| [DOC-277](DOC-277-agentic-access-docs.md) | Agentic access docs & MCP setup guide | Medium | S | BE-274, FE-276 | Planned |
| [QA-278](QA-278-agentic-access-tests.md) | Agentic access security & integration tests | Critical | M | BE-271..275 | Planned |

## Dependency Graph

```
BE-271 (agent principal)
  └── BE-272 (mcp:* abilities + REST block)
        └── BE-273 (per-tool capability gating)
              ├── BE-274 (management API + toggle)
              │     ├── FE-276 (settings UI)
              │     │     └── DOC-277 (docs)
              │     └── BE-275 (audit logging)
              └── QA-278 (security + integration tests)  ← gates the phase
```

## Out of Scope

- SSH-key issuance / SSH-as-auth for agents (token is the credential).
- Per-agent rate limiting / quotas on MCP calls (possible follow-up).
- A bespoke agent-activity timeline UI (link to the existing audit log instead).
- Non-MCP automation surfaces (the agent reaches Icefall only through MCP).
- OAuth/OIDC federation for agents.

## Security Posture (why this is safe by construction)

- **Default off**, admin-only management, global master switch.
- **Least privilege:** read-only default; deploy/write/admin are deliberate
  opt-ins.
- **MCP-only:** an agent token holds no REST abilities, so new REST routes can
  never widen its reach.
- **Auto-coverage:** every MCP tool maps to a capability group; an unmapped tool
  fails CI (QA-278).
- **Revocable & auditable:** disable/delete are immediate kill switches; every
  agent action is logged with secrets redacted.
- **No interactive auth:** agents can't log in, so there's no session/2FA attack
  surface for them.
