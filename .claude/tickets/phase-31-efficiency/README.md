# Phase 31 — Binary / dashboard / runtime efficiency

Audit (2026-06-15) of opportunities to shrink the binary and reduce runtime
overhead. Findings were **verified against the code**, not taken on trust — the
estimates below are corrected/deflated from the raw audit where it over-counted.

Baseline: release binary ~20 MB *after* IF-255 embedded the dashboard (~15 MB
before). Release profile already has `lto = true`, `codegen-units = 1`,
`strip = true`.

## Tickets (ranked by verified impact × confidence)

| Ticket | What | Est. impact | Effort | Status |
|---|---|---|---|---|
| [IF-257](IF-257-remove-utoipa.md) | Remove unused `utoipa` + `utoipa-swagger-ui` | swagger-ui/rust-embed/zip removed | S | ✅ merged (#96) |
| [IF-256](IF-256-trim-embedded-dashboard.md) | Embed only `.br`, drop `.gz` | dist 4.4→3.6 MB | S | ✅ merged (#97) |
| [IF-258](IF-258-release-profile-panic-abort.md) | `panic = "abort"` in release profile | −150–250 KB | S | ✅ merged (#97) |
| [IF-260](IF-260-metrics-collection-efficiency.md) | Reuse `System`/`Disks`, drop the 200 ms metrics sleep | runtime | M | ✅ done (#98), verified vs `top` |
| [IF-259](IF-259-tokio-granular-features.md) | `tokio` full → granular feature set | small | S | ⬜ open |

Net binary across phase-30/31: embedding the dashboard (IF-255) pushed it ~15→20 MB,
then IF-257 + IF-256 + IF-258 brought it back to **16.31 MB** — the dashboard is
embedded for ~1.3 MB net over the old non-embedded binary.

## Corrections to the raw audit (skepticism applied)

- The audit listed "remove `.gz` AND uncompressed" as two separate −1.5 MB items
  (its #3 and #12) — that's **double-counting the same files**. Folded into one
  ticket (IF-256), and we keep identity (some clients send no `Accept-Encoding`)
  while dropping `.gz` (br covers ~99% of accept-encoding cases; gzip can be done
  on the fly by the IF-252 layer for the rare identity-only client).
- `opt-level = "z"`/"s" was suggested — **not** ticketed by default: it trades
  request latency for size and needs measurement; `panic = "abort"` is the safe,
  unambiguous profile win.
- Duplicate-crate dedup (rand/nom/hmac) is mostly **transitive crypto deps**
  (ed25519-dalek, argon2, rcgen) with different MSRVs — low ROI, high churn,
  not ticketed.
- `qrcode` / `lettre` feature-gating: real but small (~100–250 KB each) and adds
  build-matrix complexity (a feature-gated 2FA/SMTP); deferred — note here, not a
  ticket unless size becomes critical.
- chrono trimming: marginal; not ticketed.

## Sequencing

IF-256 depends on IF-255 being merged (it edits the embed). IF-257 / IF-258 are
independent and can land anytime. IF-260 is runtime (not size) and independent.
