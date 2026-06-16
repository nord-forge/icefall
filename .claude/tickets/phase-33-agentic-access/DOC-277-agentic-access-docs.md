# DOC-277: Agentic access documentation & MCP setup guide

**Phase:** 33
**Priority:** Medium
**Size:** S
**Dependencies:** BE-274, FE-276

## Description

Document the agentic access feature end-to-end: what an agent is, why it's a
scoped service account rather than a shared human login, how to create one, how
to wire its token into an MCP client (Claude Code / Claude Desktop), and the
security model (capabilities, MCP-only restriction, audit, revocation).

## Changes

- New how-to guide `website/src/content/docs/guides/agentic-access.mdx`:
  - Concept: the agent principal, default-off, human-in-control framing.
  - Step-by-step: enable the toggle → create an agent → choose capabilities →
    copy the one-time token → configure it in an MCP client.
  - Capability reference: what `mcp:read` / `mcp:deploy` / `mcp:write` /
    `mcp:admin` each grant.
  - Revocation & rotation, and where to find agent actions in the audit log.
- Extend the MCP reference (`api/mcp-tools` from IF-197) to note which tool
  belongs to which capability group, kept in sync with BE-273's mapping.
- Add a `concepts/security.mdx` subsection (or the security concept page) on
  non-human principals and least-privilege for agents.
- Register the new guide in the Starlight sidebar (`website/astro.config.mjs`).

## Acceptance Criteria

- The guide builds and appears in the docs sidebar.
- A reader can follow it to create an agent and connect an MCP client without
  prior knowledge of the internals.
- The capability table matches the BE-273 tool→group mapping (no drift).
- The security model (MCP-only, opt-in writes, audit, revoke) is stated
  explicitly.

## Out of Scope

- Auto-generating the tool→capability table from code (manual table is fine for
  this phase; note the source of truth is BE-273).

## Security Notes

Docs must steer admins toward least privilege: lead with read-only, present
deploy/write/admin as deliberate grants, and emphasize that the token is a
secret shown once.
