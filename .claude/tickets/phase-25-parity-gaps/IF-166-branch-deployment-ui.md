# IF-166: Branch-specific deployment UI

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** S

## Description

The webhook receiver (IF-012) already supports branch routing, but the app settings UI doesn't expose which branch triggers auto-deploy. Add a "Deploy branch" field so users can configure which branch triggers automatic deployments.

## Acceptance Criteria

- [ ] App settings tab: "Deploy branch" text input (default: `main`)
- [ ] Stored in the existing `branch` field on the apps table
- [ ] Webhook receiver only triggers deploy when the pushed branch matches the configured branch
- [ ] Branch autocomplete: fetch branches from `git ls-remote --heads` (cached 5 minutes)
- [ ] Display the configured branch in the auto-deploy section alongside the webhook URL
- [ ] "Deploy any branch" checkbox option (wildcards: `*` deploys on any push)
- [ ] Preview environments use their own branch pattern (IF-063), not this field

## Dependencies

- IF-012 (Webhook receiver)
- IF-062 (Auto-deploy toggle)
