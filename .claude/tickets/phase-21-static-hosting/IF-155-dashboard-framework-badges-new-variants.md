# IF-155: Dashboard framework badges for new variants

**Phase:** 21 — Static Hosting Expansion
**Priority:** Low
**Estimate:** S
**Dependencies:** IF-147

## Description

The dashboard displays framework badges/labels for apps. Add display names, icons, and color coding for the new framework variants added in IF-147, IF-148, and IF-149 so users can see which framework was detected at a glance.

## Acceptance Criteria

- [ ] Framework display names map correctly:
  - `vite-svelte` → "Svelte"
  - `vite-solid` → "SolidJS"
  - `vite-preact` → "Preact"
  - `vite-generic` → "Vite"
  - `angular` → "Angular"
  - `gatsby` → "Gatsby"
  - `sveltekit` → "SvelteKit"
  - `remix` → "Remix"
- [ ] Deploy mode indicator shows "Static" vs "Container" for frameworks with both modes
- [ ] Unknown/new framework values don't crash the UI (fallback to raw string)

## Out of Scope

- Framework logos/icons (use text badges for now)
- Framework-specific settings panels

## Files to Modify

- Dashboard components that display framework info (likely in `dashboard/` directory)
