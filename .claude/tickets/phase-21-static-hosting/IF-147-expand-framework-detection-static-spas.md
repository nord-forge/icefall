# IF-147: Expand framework detection for static SPAs

**Phase:** 21 — Static Hosting Expansion
**Priority:** High
**Estimate:** M
**Dependencies:** None

## Description

The framework detection in `common/src/build/detect.rs` currently recognizes Vite+React and Vite+Vue as native-eligible (static) deploy targets, but misses several popular frameworks that also produce static output. Apps built with Svelte (Vite), SolidJS (Vite), Angular, Gatsby, and generic Vite setups are not detected properly — they either fall through to Docker unnecessarily or get misclassified as `NodeApp`/`StaticSite`.

Add new `Framework` variants and detection logic so these SPAs are correctly identified and routed through the native (Caddy file_server) pipeline instead of requiring a Docker container.

## Acceptance Criteria

### New Framework variants in `common/src/build/mod.rs`
- [ ] `ViteSvelte` — Vite + `svelte` dependency
- [ ] `ViteSolid` — Vite + `solid-js` dependency
- [ ] `VitePreact` — Vite + `preact` dependency (currently lumped into generic)
- [ ] `ViteGeneric` — Vite detected but no recognized UI library (vanilla TS, Lit, etc.)
- [ ] `Angular` — `@angular/core` dependency or `angular.json` present
- [ ] `Gatsby` — `gatsby` dependency or `gatsby-config.js`/`gatsby-config.ts` present
- [ ] `SvelteKit` — `@sveltejs/kit` dependency (handled separately in IF-148)

### Detection logic in `common/src/build/detect.rs`
- [ ] `detect_framework()` checks for `svelte` dep + Vite presence → `ViteSvelte`
- [ ] `detect_framework()` checks for `solid-js` dep + Vite presence → `ViteSolid`
- [ ] `detect_framework()` checks for `preact` dep + Vite presence → `VitePreact`
- [ ] `detect_framework()` falls back to `ViteGeneric` when Vite is detected but no known UI lib found
- [ ] `detect_framework()` checks for `@angular/core` or `angular.json` → `Angular`
- [ ] `detect_framework()` checks for `gatsby` dep → `Gatsby`
- [ ] Detection order preserves existing priority (Dockerfile > Astro > Next > Nuxt > Vite variants > Angular > Gatsby > NodeApp > StaticSite)

### Framework defaults in `common/src/build/detect.rs`
- [ ] `ViteSvelte`: build=`{pm} run build`, output=`dist`, port=80
- [ ] `ViteSolid`: build=`{pm} run build`, output=`dist`, port=80
- [ ] `VitePreact`: build=`{pm} run build`, output=`dist`, port=80
- [ ] `ViteGeneric`: build=`{pm} run build`, output=`dist`, port=80
- [ ] `Angular`: build=`{pm} run build`, output=`dist/{project-name}/browser` (detect from `angular.json`), port=80
- [ ] `Gatsby`: build=`{pm} run build`, output=`public`, port=80

### Native eligibility in `src/deploy/native.rs`
- [ ] `should_use_native()` returns `true` for: `ViteSvelte`, `ViteSolid`, `VitePreact`, `ViteGeneric`, `Angular`, `Gatsby`

### Tests
- [ ] Unit test: detects `ViteSvelte` from package.json with `svelte` + `vite`
- [ ] Unit test: detects `ViteSolid` from package.json with `solid-js` + `vite`
- [ ] Unit test: detects `VitePreact` from package.json with `preact` + `vite`
- [ ] Unit test: detects `ViteGeneric` from package.json with only `vite`
- [ ] Unit test: detects `Angular` from `angular.json`
- [ ] Unit test: detects `Gatsby` from package.json with `gatsby`
- [ ] Unit test: `should_use_native()` returns true for all new static variants
- [ ] Unit test: detection priority — Dockerfile still wins over everything

## Out of Scope

- SvelteKit adapter detection (IF-148)
- Remix SPA mode detection (IF-149)
- Dashboard UI changes for new framework types
- Dockerfile generation for new variants (IF-150)

## Files to Modify

- `common/src/build/mod.rs` — add enum variants, Display impl
- `common/src/build/detect.rs` — detection logic + framework_defaults + tests
- `src/deploy/native.rs` — expand `should_use_native()`
