# BE-275: Audit logging for agent actions

**Phase:** 33
**Priority:** High
**Size:** S
**Dependencies:** BE-273, BE-274

## Description

Every action an agent takes through MCP must be attributable and reviewable, so
the human stays informed as well as in control. Use the existing `audit_log`
table (IF-145) to record each MCP tool call made by an agent principal, plus
lifecycle events (agent created/disabled/deleted, token rotated).

## Changes

- In the MCP dispatch path (BE-273), after authorization succeeds, write an
  `audit_log` entry when the caller is an agent (`User::is_agent`):
  - `user_id` = agent id, `action` = `"mcp.tool_call"`,
    `details` = JSON `{ tool, arguments_summary, result: ok|error }`.
  - Redact secrets in `arguments_summary` (reuse the existing env-var/secret
    redaction used by the log pipeline) — never log token/password/env values.
- Lifecycle events from BE-274 also write audit entries: `agent.created`,
  `agent.updated`, `agent.disabled`, `agent.deleted`, `agent.token_rotated`,
  with the acting **admin's** user_id and the target agent in `details`.
- The agentic toggle flip writes `agentic_access.enabled` / `.disabled`.
- No new schema — only writes against the existing `audit_log` and reuses
  `create_audit_log`.

## Acceptance Criteria

- Given an agent calls `deploy_app`, then an `mcp.tool_call` audit entry exists
  with the agent's user_id, the tool name, and an outcome, and contains no secret
  values.
- Given an admin disables an agent, then an `agent.disabled` entry exists naming
  the acting admin and the target agent.
- Human (non-agent) MCP calls are not spammed into the audit log by this ticket
  (scope: agent actions only), unless an existing audit hook already covers them.
- Existing audit pruning (90-day) applies unchanged.

## Out of Scope

- A dedicated agent-activity view in the UI (FE-276 may surface a link to the
  existing audit log; a bespoke timeline is a follow-up).
- Real-time alerting on agent actions.

## Security Notes

Auditability is a control, not just observability: it makes agent behavior
reviewable after the fact and supports incident response if a token leaks.
Redaction is mandatory — the audit log must never become a secret sink.
