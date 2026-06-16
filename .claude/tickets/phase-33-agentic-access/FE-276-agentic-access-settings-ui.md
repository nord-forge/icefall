# FE-276: Agentic Access settings section

**Phase:** 33
**Priority:** High
**Size:** M
**Dependencies:** BE-274

## Description

A new **Agentic Access** section in the global Settings page where an admin turns
the feature on (default off), creates agents, picks their MCP capabilities, and
copies the one-time token. Follows the existing settings-section pattern
(`dashboard/src/islands/settings/SettingsPage/components/`, e.g.
`McpServerSection`).

## Changes

- New `AgenticAccessSection.tsx` registered in `SettingsPage.tsx`:
  - Master toggle "Enable agentic access" (default off). When off, the agent
    list/forms are visually disabled with an explanation.
  - Agent list: name, capabilities (as badges), enabled state, last used.
  - "Add agent" form: name input + a **capability picker** grouped as
    Read / Deploy / Write / Admin, defaulting to Read only; Deploy/Write/Admin
    are unchecked and clearly marked as higher-risk (write opt-in). Reuse the
    `TokenAbilityPicker` pattern from the token UIs (IF-168) but filtered to the
    `mcp:*` family.
  - On create: show the plaintext token **once** in a copy-once dialog with a
    "you won't see this again" warning; never refetch it.
  - Per-agent actions: enable/disable toggle, "Rotate token" (copy-once dialog),
    "Delete" (ConfirmDialog with consequence text).
  - Surface a link to the audit log filtered to that agent (data from BE-275).
- API client methods in `dashboard/src/lib/api.ts` for the BE-274 endpoints.
- Types in `dashboard/src/lib/types.ts` for `Agent` and the capability list.

## Acceptance Criteria

- Given agentic access is off, when the admin opens Settings, then the section
  shows the toggle off and the agent management UI is disabled.
- Given the admin creates an agent with only Read checked, then the agent is
  created with `["mcp:read"]` and the token is shown exactly once.
- Given the admin opens an existing agent, then the token is never shown again;
  only "rotate" produces a new one.
- Given the admin clicks Delete, then a confirmation describes the consequence
  before deletion.
- a11y (WCAG 2.2 AA): the toggle and every capability checkbox have visible
  labels and accessible names; the one-time-token dialog is a focus-trapped
  modal with an accessible name; the copy button has an `aria-label`; status
  messages use the existing `aria-live` region. Run `a11y-hawk` before done and
  annotate fixes at the change site.

## Out of Scope

- Backend endpoints — BE-274.
- A bespoke agent-activity timeline (link to existing audit log is enough here).

## Security Notes

The UI must make capability risk legible: read is safe-by-default, and
deploy/write/admin are presented as deliberate opt-ins so an admin doesn't
over-grant by reflex. The copy-once token must never be persisted in client
state beyond the dialog.
