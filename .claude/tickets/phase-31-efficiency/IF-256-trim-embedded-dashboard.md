# IF-256: Trim the embedded dashboard (drop `.gz`)

**Phase:** 31 — Efficiency
**Priority:** Medium
**Estimate:** S

## Description

IF-255 embedded `dashboard/dist` into the binary, including **three** copies of
every compressible asset: identity, `.br`, and `.gz`. That's the +5 MB the binary
grew. We can drop the `.gz` variants from the *embed* with negligible downside:

- **Brotli** is accepted by ~99% of clients that send `Accept-Encoding` (every
  modern browser). The embedded handler already prefers `.br`.
- The rare client that sends *only* `gzip` (or no `Accept-Encoding`) can be
  served the **identity** asset, which the IF-252 `CompressionLayer` then gzips
  on the fly. So we still serve gzip when needed — just not from a precompressed
  embedded copy.

Keep identity (needed for the layer fallback + non-`br` clients) and `.br` (the
common, zero-CPU path). Dropping `.gz` removes ~1/3 of the compressed embed.

Measured today: `dashboard/dist` precompress produced 112 `.br` + 112 `.gz`; the
`.gz` set is ~the same size as `.br` (~466 KB), so dropping it saves roughly that
much *compressed* — but the embed cost is the raw bytes, so confirm the on-disk
delta when implementing.

## Acceptance Criteria

- [ ] The precompress build step (`dashboard/scripts/precompress.mjs`) still
      produces `.br` (and identity); decide whether to stop emitting `.gz` there
      or to exclude `.gz` only from the embed.
      - Note: the on-disk `.gz` is still useful if anyone serves `dist` via the
        `ICEFALL_DASHBOARD_DIR` dev override or a static host — prefer excluding
        `.gz` from the *embed* (a `Dir` filter / include glob) over not building
        it, OR accept dropping it everywhere if simpler.
- [ ] `assets::serve` falls back correctly: `Accept-Encoding: gzip` (no br) now
      serves identity, and the IF-252 layer compresses it → client still gets
      gzip, just not precompressed.
- [ ] Binary size re-measured and recorded (expect ~20 MB → ~18 MB).
- [ ] e2e: br client gets `.br`; gzip-only client gets `content-encoding: gzip`
      (from the layer); identity client gets identity.

## Technical Notes

- `include_dir!` embeds the whole dir; filtering `.gz` out of the *embed* may
  require building `dist` without `.gz`, or post-processing. Simplest is to make
  `precompress.mjs` emit `.br` only (and keep gzip-on-the-fly via the IF-252
  layer for the rare case).
- Verify the IF-252 `CompressionLayer` still kicks in for identity responses
  from `assets::serve` (it skips responses that already set `Content-Encoding`;
  an identity response doesn't, so it will compress). Confirm with a curl.

## Dependencies

- IF-255 (embedding) must be merged first.
