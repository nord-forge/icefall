# IF-184: MCP Deploy Copilot — conversational multi-step deploys

**Phase:** 25 — Icefall+
**Priority:** High
**Estimate:** M

## Description

Extend the MCP server with a high-level `deploy_workflow` tool that orchestrates multi-step deploy sequences conversationally. An AI agent can say "deploy staging, run migrations, check health, promote to production if green, roll back if not" and Icefall executes the entire workflow, reporting progress at each step.

## Acceptance Criteria

- [ ] New MCP tool: `deploy_workflow` accepting a structured plan:
  ```json
  {
    "steps": [
      { "action": "deploy", "app": "my-api", "branch": "staging" },
      { "action": "exec", "app": "my-api", "command": "npx prisma migrate deploy" },
      { "action": "health_check", "app": "my-api", "timeout_secs": 60 },
      { "action": "promote", "from": "my-api-staging", "to": "my-api-production" },
      { "action": "rollback_if_unhealthy", "app": "my-api-production", "timeout_secs": 120 }
    ]
  }
  ```
- [ ] Each step reports progress back to the MCP client as structured events
- [ ] If any step fails: halt the workflow and return the failure context
- [ ] Conditional steps: `rollback_if_unhealthy` only triggers if the health check fails
- [ ] Dry-run mode: validate the workflow plan without executing
- [ ] New MCP tool: `diagnose` — given an app name, pull logs (last 50 lines), health status, resource usage, recent deploys, and return a structured diagnostic summary
- [ ] New MCP tool: `suggest_fix` — given a diagnostic, suggest next actions (restart, rollback, scale up, check env vars)
- [ ] All new tools respect the existing MCP role-based permissions

## Technical Notes

- The workflow orchestrator chains existing internal functions — it's not calling the MCP tools recursively, it calls the underlying Rust functions directly
- Progress events use the MCP protocol's notification mechanism
- The `diagnose` tool is the MCP equivalent of "ssh in and poke around" — it's the killer demo for AI-assisted ops

## Dependencies

- IF-044 (MCP server — existing 13 tools)
- IF-163 (Post-deploy commands — for the `exec` step)
