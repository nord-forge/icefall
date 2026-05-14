# IF-195: MCP workflow orchestration tools

**Phase:** 27 — MCP Expansion
**Priority:** High
**Estimate:** M

## Description

Expand the MCP server from 13 individual tools to a comprehensive orchestration toolkit. Add high-level workflow tools (`deploy_workflow`, `diagnose`, `suggest_fix`), bulk operations, and resource creation tools so AI agents can manage Icefall end-to-end.

## Acceptance Criteria

### Workflow Tools
- [ ] `deploy_workflow` — execute multi-step deploy plans (deploy → exec → health → promote/rollback)
- [ ] `diagnose` — pull logs, health, metrics, recent deploys for an app and return structured diagnostic
- [ ] `suggest_fix` — given a diagnostic, suggest actionable next steps
- [ ] `rollback_if_unhealthy` — conditional rollback based on health check status

### Bulk Operations
- [ ] `bulk_restart` — restart multiple apps by name, tag, or project
- [ ] `bulk_deploy` — deploy all apps in a project or matching a tag
- [ ] `bulk_env_set` — set an env var across multiple apps

### Resource Creation
- [ ] `create_app` — full app creation (repo URL, branch, build settings, env vars) in one call
- [ ] `create_database` — provision a managed database with type, name, credentials
- [ ] `add_domain` — add a domain to an app with optional path routing
- [ ] `setup_webhook` — configure auto-deploy webhook for an app

### Server Management
- [ ] `server_status` — get all servers with health, metrics, and app counts
- [ ] `migrate_app` — move an app between servers
- [ ] `server_optimize` — get optimization suggestions (from IF-191)

### Utility
- [ ] `tunnel` — open a secure tunnel to a container port (for agent-based tools)
- [ ] `export_bundle` — export an app as a `.icefall` bundle
- [ ] `import_bundle` — import a `.icefall` bundle

### Protocol
- [ ] All tools return structured JSON responses with consistent schema
- [ ] Progress notifications for long-running operations (deploy, bulk ops)
- [ ] Error responses include error code, message, and suggested action
- [ ] Role-based access: viewer can read, deployer can deploy, admin can create/delete

## Technical Notes

- The workflow tools chain existing internal functions — they don't call MCP tools recursively
- Progress events use MCP protocol's notification mechanism
- Total tool count after expansion: ~30 tools (from current 13)

## Dependencies

- IF-044 (MCP server — current 13 tools)
- IF-184 (MCP Deploy Copilot — the `deploy_workflow` tool)
