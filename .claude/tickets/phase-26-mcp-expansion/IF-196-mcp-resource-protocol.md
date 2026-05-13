# IF-196: MCP resources and prompts protocol

**Phase:** 26 — MCP Expansion
**Priority:** Medium
**Estimate:** M

## Description

Implement MCP's resource and prompt protocol in addition to tools. Resources let AI agents browse apps, databases, and configs as structured data. Prompts provide pre-built templates for common operations. This makes Icefall the most complete MCP server in the PaaS space.

## Acceptance Criteria

### Resources (read-only data the AI can browse)
- [ ] `icefall://apps` — list all apps with status, server, last deploy
- [ ] `icefall://apps/{id}` — full app detail including config, env vars (masked), domains
- [ ] `icefall://apps/{id}/logs` — recent log lines (last 100)
- [ ] `icefall://databases` — list all databases with type, status, linked apps
- [ ] `icefall://servers` — list all servers with health, metrics, app counts
- [ ] `icefall://servers/{id}/metrics` — current resource usage
- [ ] `icefall://deploys/recent` — last 10 deploys across all apps
- [ ] `icefall://incidents` — active incidents (if IF-178 is built)
- [ ] `icefall://settings` — current platform settings (non-sensitive)

### Prompts (pre-built templates)
- [ ] `deploy-app` — guided deploy with app selection and branch/tag input
- [ ] `create-app` — step-by-step app creation with framework detection
- [ ] `troubleshoot` — structured diagnostic prompt: "describe the issue" → pull logs/health/metrics → suggest fix
- [ ] `migrate-server` — guided app migration between servers
- [ ] `setup-monitoring` — configure health checks and notification alerts for an app
- [ ] `security-audit` — check for common issues: exposed ports, missing resource limits, old images

### Protocol Compliance
- [ ] Resources support `list` and `read` operations per MCP spec
- [ ] Prompts support `list` and `get` with argument schemas
- [ ] Resource subscriptions: AI clients can subscribe to resource changes (SSE-backed)
- [ ] All resources respect role-based access (viewer sees masked secrets)

## Technical Notes

- MCP resources map naturally to the existing REST API — each resource is a read-only view of an API endpoint
- Prompts are essentially structured instructions that guide the AI through a multi-tool workflow
- Resource subscriptions can piggyback on the existing SSE infrastructure

## Dependencies

- IF-044 (MCP server)
- IF-195 (MCP workflow tools — prompts reference tools)
