# IF-197: MCP integration guides (Claude Code, Cursor, Windsurf)

**Phase:** 26 — MCP Expansion
**Priority:** High
**Estimate:** S

## Description

Write comprehensive integration guides for using Icefall's MCP server with popular AI coding tools. Include setup instructions, example workflows, and .mcp.json configs for each tool.

## Acceptance Criteria

### Claude Code Guide
- [ ] `.mcp.json` config for connecting Claude Code to Icefall MCP
- [ ] Step-by-step setup: API token creation, MCP server URL, auth config
- [ ] Example workflows: "deploy from terminal", "check app status", "rollback a deploy"
- [ ] Advanced: using `deploy_workflow` for multi-step operations
- [ ] Troubleshooting: common connection issues, auth errors

### Cursor Guide
- [ ] MCP settings.json configuration for Cursor
- [ ] Setup walkthrough with screenshots
- [ ] Example: "deploy this project" from within the editor
- [ ] Integration with Cursor's chat for conversational deploys

### Windsurf Guide
- [ ] MCP configuration for Windsurf (Codeium)
- [ ] Setup and authentication
- [ ] Example workflows

### Generic MCP Client Guide
- [ ] How to connect any MCP client to Icefall
- [ ] Auth: API token in MCP config
- [ ] Available tools, resources, and prompts reference
- [ ] JSON-RPC examples for each tool

### Documentation Pages
- [ ] `docs/api/mcp.mdx` — expanded from current stub to full reference
- [ ] `docs/guides/mcp-claude-code.mdx` — Claude Code integration
- [ ] `docs/guides/mcp-cursor.mdx` — Cursor integration
- [ ] `docs/guides/mcp-windsurf.mdx` — Windsurf integration
- [ ] `docs/guides/mcp-generic.mdx` — generic MCP client

## Dependencies

- IF-044 (MCP server)
- IF-195 (MCP workflow tools — document the new tools)
