# IF-150: Dockerfile generation for new framework variants

**Phase:** 21 — Static Hosting Expansion
**Priority:** Medium
**Estimate:** M
**Dependencies:** IF-147, IF-148, IF-149

## Description

When the new framework variants (IF-147, IF-148, IF-149) are deployed in Docker mode (either explicitly via `deploy_mode: "container"` or because the SSR variant was detected), the build orchestrator needs to generate appropriate Dockerfiles. Add Dockerfile templates for each new framework.

Static-only variants reuse the existing `dockerfile_static_build()` pattern (multi-stage: build with Node, serve with Caddy). SSR variants need framework-specific runtime stages.

## Acceptance Criteria

### Static variants in `common/src/build/dockerfile.rs`
- [ ] `ViteSvelte`, `ViteSolid`, `VitePreact`, `ViteGeneric` → delegate to `dockerfile_static_build()` (same as ViteReact/ViteVue)
- [ ] `Angular` → `dockerfile_static_build()` with correct output dir from detection
- [ ] `Gatsby` → `dockerfile_static_build()` with `output=public`

### SSR variants
- [ ] `SvelteKit` (SSR) → multi-stage Dockerfile:
  - Build stage: install + `{pm} run build`
  - Runtime stage: `node:XX-slim`, copy `build/` dir, `node build/index.js`
  - Non-root user `sveltekit`
- [ ] `Remix` (SSR) → multi-stage Dockerfile:
  - Build stage: install + `{pm} run build`
  - Runtime stage: `node:XX-slim`, copy `build/` dir, `remix-serve ./build/server/index.js`
  - Non-root user `remix`

### `generate_dockerfile()` match arms
- [ ] All new `Framework` variants are handled (no compiler warnings for non-exhaustive match)
- [ ] SSR/SPA routing: SvelteKit SSR and Remix SSR get their own templates; static variants use `dockerfile_static_build()`

### Tests
- [ ] Generates valid Dockerfile for each new static variant
- [ ] Generates valid Dockerfile for SvelteKit SSR
- [ ] Generates valid Dockerfile for Remix SSR
- [ ] All Dockerfiles include non-root USER directive
- [ ] All Dockerfiles expose correct port

## Out of Scope

- Custom Dockerfile support (already handled by `Framework::Dockerfile`)
- Bun runtime images for SSR variants

## Files to Modify

- `common/src/build/dockerfile.rs` — new match arms + template functions
