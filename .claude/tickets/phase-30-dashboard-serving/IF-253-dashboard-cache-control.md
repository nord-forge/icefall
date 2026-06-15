# IF-253: Cache-Control headers for dashboard assets

**Phase:** 30 — Dashboard Serving
**Priority:** High
**Estimate:** S

## Description

`ServeDir` sends **no `Cache-Control` headers** for the dashboard, so the browser
revalidates/refetches assets on every visit — even though the `_astro/*` files
are **content-hashed and immutable** (e.g. `Alert.CXSNITkF.js`). Adding correct
caching turns repeat dashboard loads into near-instant (assets served from disk
cache, only the small HTML shell fetched).

Pairs naturally with [IF-252](IF-252-dashboard-http-compression.md) — do them in
one PR.

## Acceptance Criteria

- [ ] `_astro/*` (content-hashed assets) →
      `Cache-Control: public, max-age=31536000, immutable`.
- [ ] HTML shells (`index.html`, prerendered route shells) →
      `Cache-Control: no-cache` (or a short max-age) so a deploy/update is picked
      up immediately — the hashed asset URLs inside change on every build.
- [ ] `csp-hashes.json` is not user-facing; ensure it isn't served with a long
      cache that masks a rebuild (it's read server-side, not by the browser, so
      likely moot — confirm it isn't exposed via `ServeDir`).
- [ ] Verify with `curl -I` that the headers differ between `/_astro/x.js` and
      `/` (the shell).

## Technical Notes

- Apply via a `SetResponseHeaderLayer` keyed on path, or a small middleware that
  inspects the response path/extension. `tower-http`'s `set-header` feature is
  already enabled.
- Distinguish immutable assets (anything under `_astro/`) from HTML shells. The
  simplest reliable signal is the `/_astro/` path prefix.
- Interacts with the self-update flow (IF-016 era): after an update the new
  build has new hashed filenames, so `immutable` on assets is safe; the
  `no-cache` HTML shell is what lets clients discover them.

## Out of Scope

- Compression (IF-252).
- ETags/conditional requests — `ServeDir` already does Last-Modified/ETag for
  files; this ticket is about explicit `Cache-Control`.

## Dependencies

- None. Best landed together with IF-252.
