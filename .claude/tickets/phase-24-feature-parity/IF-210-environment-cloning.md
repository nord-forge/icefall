# IF-210: Environment cloning

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** M

## Description

Allow users to clone an entire environment (or a single resource) to a different project, environment, or server. This is a common workflow when setting up staging from production, or duplicating a working setup to a new server. Includes the option to clone persistent volume data.

## Acceptance Criteria

- [ ] "Clone" action on environment pages: opens a modal with target selection
- [ ] Target selection: pick destination project, environment, and server
- [ ] New name input (defaults to "{original}-copy")
- [ ] Toggle: "Clone volume data" (copies container volume contents to the target, default off)
- [ ] Cloning creates new app/database records with copied configuration (env vars, domains excluded, build config preserved)
- [ ] For single-resource clone: "Clone" option in resource operations dropdown on app/database detail
- [ ] "Move" option: reassign a resource to a different environment without recreating (updates `environment_id` FK)
- [ ] API endpoints: `POST /environments/{id}/clone`, `POST /apps/{id}/clone`, `POST /apps/{id}/move`
- [ ] Volume data clone: container cp (Docker/Podman) from source + cp into new container (or volume-to-volume copy)
- [ ] Progress feedback via SSE (cloning can be slow if volumes are large)

## Technical Notes

- Clone is a deep copy: new app record, new env vars, new volume mounts — but domains are NOT copied (would conflict)
- For multi-server clones, volume data transfer goes through the control plane (agent download → agent upload)
- Move is simpler: just update the FK, no data copy

## Out of Scope

- Cross-instance cloning (use Portable App Bundles IF-192 for that)
- Automatic domain generation for cloned resources
- Clone scheduling / automation

## Dependencies

- IF-074 (Projects)
- IF-147 (Environments per project)
