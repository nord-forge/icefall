# IF-202: Complete API reference documentation

**Phase:** 27 — Comprehensive Docs
**Priority:** High
**Estimate:** M

## Description

Generate comprehensive API reference documentation from the OpenAPI spec. Every endpoint should have a description, request/response examples, error codes, and authentication requirements. This replaces the current stub pages.

## Pages to Create/Rewrite

### `api/rest.mdx` (rewrite into multiple pages)
- [ ] `api/overview.mdx` — authentication, base URL, pagination, error format, rate limits
- [ ] `api/apps.mdx` — app CRUD, deploy triggers, start/stop/restart
- [ ] `api/deploys.mdx` — deploy list, status, rollback
- [ ] `api/databases.mdx` — database CRUD, backups, linking
- [ ] `api/domains.mdx` — domain CRUD, DNS verification
- [ ] `api/env-vars.mdx` — env var CRUD, scoping
- [ ] `api/servers.mdx` — server CRUD, enrollment, metrics
- [ ] `api/users.mdx` — user management, roles, invites
- [ ] `api/auth.mdx` — login, logout, sessions, 2FA
- [ ] `api/tokens.mdx` — API token CRUD
- [ ] `api/notifications.mdx` — notification channels, rules, test
- [ ] `api/settings.mdx` — global settings CRUD
- [ ] `api/webhooks.mdx` — webhook payload format for GitHub/GitLab

### `api/mcp.mdx` (rewrite)
- [ ] MCP server overview: what it is, how to connect
- [ ] Tool reference: every tool with parameters, return types, examples
- [ ] Resource reference: every resource URI with schema
- [ ] Prompt reference: every prompt with argument schemas
- [ ] Authentication: API token in MCP config
- [ ] Error handling: MCP error codes

### Each API Page Includes
- [ ] Endpoint URL and method
- [ ] Authentication requirement (session, token, role)
- [ ] Request body schema with field descriptions and types
- [ ] Example request (curl + JavaScript fetch)
- [ ] Example response (success + error cases)
- [ ] HTTP status codes and their meanings
- [ ] Rate limiting information

## Standards

- [ ] Generated from OpenAPI spec where possible (ensure spec is complete first)
- [ ] Every example is runnable (correct auth headers, realistic data)
- [ ] Error responses documented for each endpoint (not just 200)
- [ ] Pagination explained with cursor/offset examples

## Dependencies

- IF-036 (OpenAPI specification)
- IF-195 (MCP workflow tools — for MCP reference)
