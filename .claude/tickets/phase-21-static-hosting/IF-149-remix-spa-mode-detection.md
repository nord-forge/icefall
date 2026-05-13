# IF-149: Remix SPA mode detection

**Phase:** 21 — Static Hosting Expansion
**Priority:** Medium
**Estimate:** S
**Dependencies:** IF-147

## Description

Remix 2.5+ introduced an SPA mode (`ssr: false` in `vite.config.ts` or `remix.config.js`) that outputs a static `build/client/` directory with no server component. Detect this mode and route to the native pipeline. Standard Remix (SSR) should continue using Docker.

## Acceptance Criteria

### Detection in `common/src/build/detect.rs`
- [ ] New `Framework::Remix` variant in `common/src/build/mod.rs`
- [ ] New enum `RemixMode { Spa, Ssr }` in `common/src/build/mod.rs`
- [ ] `detect_framework()` checks for `@remix-run/react` or `@remix-run/dev` dependency → `Framework::Remix`
- [ ] `detect_remix_mode()` reads `vite.config.ts`/`vite.config.js` and `remix.config.js`:
  - Contains `ssr: false` → `RemixMode::Spa`
  - Otherwise → `RemixMode::Ssr`
- [ ] `DetectionResult` gets `remix_mode: Option<RemixMode>` field

### Framework defaults
- [ ] Remix SPA: build=`{pm} run build`, output=`build/client`, port=80, no start_command
- [ ] Remix SSR: build=`{pm} run build`, output=`build`, start_command=`remix-serve ./build/server/index.js`, port=3000

### Native eligibility
- [ ] `should_use_native()` returns `true` for `Remix` when `remix_mode == Some(Spa)`
- [ ] `should_use_native()` returns `false` for `Remix` when `remix_mode == Some(Ssr)` or `None`

### Tests
- [ ] Detects Remix SPA mode from `ssr: false` in vite config
- [ ] Detects Remix SSR as default
- [ ] Remix SPA is native-eligible
- [ ] Remix SSR is not native-eligible

## Out of Scope

- Remix SSR Dockerfile generation (IF-150)
- Remix v1 compatibility

## Files to Modify

- `common/src/build/mod.rs` — add `Remix` variant, `RemixMode` enum
- `common/src/build/detect.rs` — detection + mode detection + framework defaults
- `src/deploy/native.rs` — expand `should_use_native()`
