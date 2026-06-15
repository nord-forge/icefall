# IF-252: HTTP compression for dashboard assets

**Phase:** 30 — Dashboard Serving
**Priority:** High
**Estimate:** S

## Description

The dashboard ships **~912 KB of raw JS** (plus CSS/HTML) and the Rust server
serves it **uncompressed** — `tower-http` is compiled without the
`compression-gzip` / `compression-br` features and there is no `CompressionLayer`
in the router. Every dashboard load transfers the full uncompressed payload,
which is painful on a fresh droplet over a slow link. Gzip alone cuts JS ~70%
(~912 KB → ~270 KB); Brotli does better.

This is the single highest-impact dashboard change.

## Acceptance Criteria

- [ ] Add a `CompressionLayer` (gzip + br) to the router, applied to the
      dashboard `fallback_service` responses (not strictly needed on `/api/v1`
      JSON, but harmless there too — decide scope).
- [ ] Enable the required `tower-http` features (`compression-gzip`,
      `compression-br`) in `Cargo.toml`.
- [ ] Verify `Content-Encoding: br` (or `gzip`) is returned for `_astro/*.js`
      when the client sends `Accept-Encoding`.
- [ ] Confirm the API JSON responses still work (compression negotiates per
      `Accept-Encoding`; clients that don't ask get identity).
- [ ] No double-compression with Caddy — Caddy proxies and should pass through
      the already-encoded body (verify it doesn't re-compress or strip).

## Technical Notes

- `src/api/mod.rs::build_router` is where the layer goes (wrap the router or the
  `serve_dir`).
- `tower-http` is already a dependency (`Cargo.toml` ~line 19) — just add the
  compression features.
- Brotli has the best ratio for static text assets; include both so older
  clients fall back to gzip.
- Alternative/[complement: IF-254](IF-254-dashboard-precompressed-assets.md)
  serves precompressed files instead of compressing per-request (less CPU).
  This ticket is the zero-build-change version.

## Out of Scope

- Precompressing assets at build time (that's IF-254).
- Caching headers (IF-253).

## Dependencies

- None.
