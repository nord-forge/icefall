# IF-255: Embed the dashboard in the binary

**Phase:** 30 — Dashboard Serving
**Priority:** Medium (High DX)
**Estimate:** M

## Description

Today the dashboard is a separate `dashboard/dist` directory that must be
packaged into the release tarball, copied to `/var/lib/icefall/dashboard/dist`
by `install.sh`, and found by the daemon **relative to its cwd**
(`WorkingDirectory=/var/lib/icefall`). That works for the one-liner installer but
is fragile: anyone running the binary by hand, or writing their own systemd unit
without `WorkingDirectory`, gets a silent `dashboard not built` 404.

Embedding the built dashboard into the binary (e.g. `rust-embed` or `include_dir`)
removes the whole class of failure: there is no `dashboard/dist` to install,
copy, locate, or get wrong. The release artifact becomes **just the binary**, and
`install.sh` loses the dashboard-copy step entirely.

This is the biggest DX simplification of the four.

## Acceptance Criteria

- [ ] The `dashboard/dist` output is embedded into the `icefall` binary at build
      time (behind a build step that runs `bun run build` first).
- [ ] The server serves embedded assets (replacing `ServeDir::new("dashboard/dist")`)
      with the same routing behavior, including the dynamic-route shell fallback
      (`dashboard_fallback` / `DYNAMIC_ROUTE_PREFIXES`).
- [ ] CSP hashes: `csp-hashes.json` is currently read from disk
      (`src/api/middleware.rs`). Embed it too, or compute hashes from the embedded
      HTML, so CSP still works with no `dashboard/dist` on disk.
- [ ] Release workflow: drop the `cp -r dashboard/dist` packaging; tarball (or
      bare binary) contains only `icefall`.
- [ ] `install.sh`: remove the dashboard extraction/copy step and the
      `WorkingDirectory` dependency for serving the UI.
- [ ] Manual-install docs updated — no more "set WorkingDirectory so it finds
      dashboard/dist" caveat.
- [ ] Binary size impact documented (expect ~15 MB → ~18 MB; with IF-254's
      precompressed embed it can be smaller since you embed `.br`).

## Technical Notes

- `rust-embed` (with the `compression` feature) or `include_dir`. `rust-embed`
  has a serve-axum story and can store compressed bytes.
- **Pairs with [IF-254](IF-254-dashboard-precompressed-assets.md):** embed the
  Brotli-compressed assets and serve them with `Content-Encoding: br` — best of
  both (small binary growth + zero runtime compression). Do IF-255's design with
  IF-254 in mind.
- Trade-off: updating the dashboard now requires a binary rebuild/redeploy
  (already the case for the self-update flow, which ships a new binary anyway —
  confirm the updater replaces the whole binary, so this is consistent).
- Keep an escape hatch? Optionally allow an on-disk `dashboard/dist` override
  (env var) for local UI development without rebuilding the binary — nice for DX.

## Out of Scope

- Compression/caching behavior of the embedded assets beyond wiring
  `Content-Encoding` (covered by IF-252/IF-254).

## Dependencies

- Decide before IF-254 (precompression has to move into the embed step).
- Touches the self-update flow — verify the updater ships the whole binary.
