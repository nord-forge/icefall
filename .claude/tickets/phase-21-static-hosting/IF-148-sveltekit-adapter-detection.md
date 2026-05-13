# IF-148: SvelteKit adapter detection (static vs SSR)

**Phase:** 21 — Static Hosting Expansion
**Priority:** High
**Estimate:** S
**Dependencies:** IF-147

## Description

SvelteKit can produce either static output (via `@sveltejs/adapter-static`) or an SSR Node server (via `@sveltejs/adapter-node`). This is analogous to the Astro static/SSR split already implemented. Detect which adapter is in use and route to the correct deploy pipeline — native for static, Docker for SSR.

## Acceptance Criteria

### SvelteKit mode detection in `common/src/build/detect.rs`
- [ ] New enum `SvelteKitMode { Static, Ssr }` in `common/src/build/mod.rs`
- [ ] `detect_sveltekit_mode()` reads `svelte.config.js`/`svelte.config.ts`:
  - Contains `adapter-static` → `SvelteKitMode::Static`
  - Contains `adapter-node`, `adapter-vercel`, `adapter-netlify`, `adapter-auto` → `SvelteKitMode::Ssr`
  - No adapter found → default to `SvelteKitMode::Ssr` (safe fallback)
- [ ] `DetectionResult` gets `sveltekit_mode: Option<SvelteKitMode>` field
- [ ] `detect_framework()` checks for `@sveltejs/kit` dependency → `Framework::SvelteKit`

### Framework defaults
- [ ] SvelteKit Static: build=`{pm} run build`, output=`build`, port=80, no start_command
- [ ] SvelteKit SSR: build=`{pm} run build`, output=`build`, start_command=`node build/index.js`, port=3000

### Native eligibility
- [ ] `should_use_native()` returns `true` for `SvelteKit` when `sveltekit_mode == Some(Static)`
- [ ] `should_use_native()` returns `false` for `SvelteKit` when `sveltekit_mode == Some(Ssr)` or `None`

### Tests
- [ ] Detects SvelteKit with `adapter-static` as static mode
- [ ] Detects SvelteKit with `adapter-node` as SSR mode
- [ ] Detects SvelteKit with no adapter config as SSR (safe default)
- [ ] SvelteKit Static is native-eligible
- [ ] SvelteKit SSR is not native-eligible
- [ ] SvelteKit detection takes priority over generic `ViteSvelte`

## Out of Scope

- SvelteKit SSR Dockerfile generation (IF-150)
- SvelteKit with custom adapters

## Files to Modify

- `common/src/build/mod.rs` — add `SvelteKit` variant, `SvelteKitMode` enum
- `common/src/build/detect.rs` — `detect_sveltekit_mode()`, framework defaults, detection priority
- `src/deploy/native.rs` — expand `should_use_native()` for SvelteKit
