# IF-261: Settings UI alignment & section consistency

**Phase:** 18 — UX Polish
**Priority:** Medium
**Estimate:** S

## Description

Three layout/consistency issues in the Settings page sections, found during
beta testing:

1. **General section** — the timezone dropdown (right column, under Base
   Domain) has no field label, unlike every other input on the page, and it
   does not align with the Recovery Email input in the left column. It floats
   without a label and sits at an inconsistent vertical position.

2. **Instance Backup section** — the three controls (Enable Scheduled Backups
   toggle, Schedule dropdown, Retention Count input) are laid out unevenly:
   the toggle and the two inputs do not line up. The two inputs beside the
   switch should be aligned with each other in a consistent grid.

3. **Reverse Proxy section** — this section is styled differently from the
   other settings cards (different header treatment, spacing, and control
   styling). Bring it up to visual consistency with the General / Instance
   Backup / Container Cleanup cards (matching card header with icon, label
   styling, input styling, button row).

## Acceptance Criteria

- [ ] Timezone field has a visible label (e.g. "Timezone") matching the label
      style of other fields, and aligns horizontally with the Recovery Email
      field in the opposite column.
- [ ] Instance Backup: the Schedule and Retention Count inputs align with each
      other on a shared baseline/grid; the toggle + inputs read as an even,
      intentional layout.
- [ ] Reverse Proxy section uses the same card/header/label/input/button
      styling as the other settings sections (consistent icon header, field
      labels, control styling).
- [ ] Light + dark mode both validated.
- [ ] a11y: every input has an associated label; alignment changes don't break
      keyboard order or focus visibility.

## Out of Scope

- No functional/behavioral changes to settings (save logic, validation,
  endpoints) — layout/styling only.
- No new settings fields.

## Dependencies

- None.
