# IF-254: Precompressed dashboard assets

**Phase:** 30 — Dashboard Serving
**Priority:** Medium
**Estimate:** M

## Description

Instead of compressing the dashboard on every request (IF-252), emit `.br` and
`.gz` variants **at build time** and let `ServeDir` serve them directly via
`precompressed_br()` / `precompressed_gzip()`. This gives the best compression
ratio (build-time Brotli at max quality) with **zero per-request CPU** — ideal
for a 1 vCPU box. The "optimize into oblivion" option.

This supersedes IF-252's per-request compression *for the dashboard* (keep
per-request only if API JSON also needs it).

## Acceptance Criteria

- [ ] Dashboard build emits `*.br` and `*.gz` alongside each compressible asset
      (JS/CSS/HTML/JSON/SVG) in `dashboard/dist`.
- [ ] `ServeDir` is configured with `.precompressed_br().precompressed_gzip()`
      so it serves the precompressed variant when the client supports it, falling
      back to identity.
- [ ] The release tarball + `install.sh` carry the `.br`/`.gz` files (they're
      already under `dashboard/dist`, which is copied wholesale — verify nothing
      filters them out).
- [ ] CI dashboard-build step produces the compressed files (so the release
      artifact has them).
- [ ] Verify `Content-Encoding` is set and the served bytes match the
      precompressed file (not re-compressed).

## Technical Notes

- Build options: the `astro-compress`/`@playform/compress` integration, or a
  small post-build script (`gzip -k`, `brotli -k`) over `dist`. Prefer the
  explicit script so it's obvious in the build and easy to reason about.
- `tower-http` `ServeDir` supports `precompressed_*` with the `fs` feature
  (already enabled).
- Decide interaction with IF-252: if both land, keep per-request compression for
  `/api/v1` JSON and precompressed-static for the dashboard, or drop IF-252 for
  the dashboard entirely.
- **Interacts with [IF-255](IF-255-embed-dashboard-in-binary.md):** if the
  dashboard is embedded in the binary, precompression must move into the embed
  step (embed the `.br` bytes). Decide IF-255 first.

## Out of Scope

- Embedding the dashboard (IF-255).

## Dependencies

- Coordinate with IF-252 (overlapping) and IF-255 (ordering).
