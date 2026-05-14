# IF-221: Force rebuild without cache

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

Add a "Force Rebuild" option that rebuilds the container image from scratch, ignoring all layer caches. Useful when a cached layer contains stale dependencies (e.g., `npm install` cached but `package.json` hasn't changed while a transitive dependency has), or when debugging build issues.

## Acceptance Criteria

- [ ] "Force Rebuild" button in app header alongside the existing "Deploy" button (dropdown/split button)
- [ ] When triggered: passes `--no-cache` to the container image build (Docker/Podman)
- [ ] Deploy log shows "Force rebuild (no cache)" indicator
- [ ] Deploy history entry tagged as "force rebuild" to distinguish from normal deploys
- [ ] Also available in the deploy dropdown menu for each app
- [ ] Per-app toggle in advanced settings: "Disable build cache" (always build without cache)
- [ ] `disable_build_cache` boolean on apps table
- [ ] API: `POST /apps/{id}/deploy` accepts `no_cache: true` parameter
- [ ] CLI: `icefall deploy --no-cache`
- [ ] MCP: `deploy_app` tool accepts `no_cache` parameter
- [ ] For multi-server: `--no-cache` flag passed to the agent's build command

## Technical Notes

- Bollard's `build_image` options already support `nocache: true` — just wire it through
- For Podman: equivalent `--no-cache` flag on build
- The "always disable cache" toggle is a persistent per-app setting; the "force rebuild" button is a one-time action

## Out of Scope

- Selective cache invalidation (clear specific layers)
- Build cache size management / pruning
- Caching between servers (registry-based caching)

## Dependencies

- IF-010 (Image build orchestrator)
- IF-132 (Agent build pipeline — for multi-server)
