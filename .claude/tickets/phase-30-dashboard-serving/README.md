# Phase 30 — Dashboard serving & DX optimization

Findings from the 2026-06-15 audit of how the Icefall dashboard is built,
packaged, installed, and served. Source investigation summary below; each
optimization is filed as its own ticket (IF-252 … IF-255).

## How it works today (end-to-end)

1. **Build** — `dashboard/` (Astro + Preact, `output: 'static'`). `bun run build`
   → `dashboard/dist/` (~2.8 MB): prerendered HTML shells + content-hashed
   JS/CSS under `_astro/`, plus `csp-hashes.json`. Dynamic routes
   (`/apps/_`, `/servers/_`, `/teams/_`, `/invitations/_`) are prerendered to
   per-prefix shells. No Node/SSR adapter.
2. **Package** (`.github/workflows/release.yml`) — release job builds the
   dashboard, `cp -r dashboard/dist staging/dashboard/dist` next to the binary,
   `tar czf`. Tarball layout: `{icefall, dashboard/dist/}`.
3. **Install** (`install.sh`) — extracts the tarball, installs the binary to
   `/usr/local/bin/icefall`, copies `dashboard/dist` →
   `/var/lib/icefall/dashboard/dist`. The systemd unit sets
   `WorkingDirectory=/var/lib/icefall`.
4. **Serve** (`src/api/mod.rs`) — `ServeDir::new("dashboard/dist")` is the Axum
   `fallback_service` (everything not under `/api/v1`). `DASHBOARD_DIST` is a
   path **relative to the process cwd**. A custom `dashboard_fallback` maps
   unmatched dynamic routes to the right prerendered shell. Caddy fronts TLS but
   does not serve these files.

## What's already good (don't touch)

- Asset names are content-hashed (`Alert.CXSNITkF.js`) → safe to cache forever.
- CSP script hashes load once via `LazyLock` (`src/api/middleware.rs`), not per
  request.
- Small footprint (2.8 MB dashboard), no SSR runtime, no Node on the server.

## Gaps (each filed as a ticket)

| Ticket | Title | Impact | Effort |
|---|---|---|---|
| [IF-252](IF-252-dashboard-http-compression.md) | HTTP compression (gzip/br) | **High** — ~912 KB raw JS served uncompressed | S | ✅ merged (#93) |
| [IF-253](IF-253-dashboard-cache-control.md) | `Cache-Control` for static assets | **High** — immutable assets refetched every visit | S | ✅ merged (#93) |
| [IF-254](IF-254-dashboard-precompressed-assets.md) | Precompressed assets + `ServeDir::precompressed_*` | Medium — zero per-request CPU | M | ✅ merged (#94) — build-time br q11, 76% |
| [IF-255](IF-255-embed-dashboard-in-binary.md) | Embed dashboard in the binary (DX) | **High DX** — removes a class of install/serve failures | M | ✅ merged (#95) — also fixed stale-UI-on-update bug |

**Phase 30 complete.** Follow-on binary-size work is tracked in
[phase-31-efficiency](../phase-31-efficiency/).

## Recommended sequencing

- **Tier 1 (do first, one small PR):** IF-252 + IF-253 together — compression +
  caching. ~30 lines, makes a fresh-droplet dashboard load dramatically snappier.
- **Tier 2 (follow-up, independent):** IF-255 (embed) is the big DX win; IF-254
  (precompressed) is the perf-purist option. IF-254 and IF-255 interact — if the
  dashboard is embedded, precompression has to move into the embed step — so
  decide IF-255 before doing IF-254.
