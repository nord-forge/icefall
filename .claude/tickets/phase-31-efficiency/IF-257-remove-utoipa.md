# IF-257: Remove unused utoipa + utoipa-swagger-ui

**Phase:** 31 — Efficiency
**Priority:** Medium
**Estimate:** S

## Description

`utoipa` and `utoipa-swagger-ui` are declared as dependencies but **completely
unused** in `src/` — verified: zero `utoipa::`, `#[derive(ToSchema)]`,
`#[utoipa::path]`, `OpenApi`, or `SwaggerUi` references anywhere. The OpenAPI
spec at `src/api/routes/openapi.rs` is **100% handcrafted** `serde_json::json!`
and serves `/openapi.json` directly.

`utoipa-swagger-ui` in particular bundles the entire Swagger UI (a large
JS/CSS payload) into the binary at build time. Both crates are pure dead weight.

This is the highest-confidence size win in the audit.

## Acceptance Criteria

- [ ] Remove `utoipa` and `utoipa-swagger-ui` from `[workspace.dependencies]`
      (Cargo.toml lines ~83-84) and `[dependencies]` (lines ~197-198).
- [ ] `cargo build --release` succeeds (nothing references them).
- [ ] `/api/v1/openapi.json` still serves the handcrafted spec unchanged.
- [ ] Binary size re-measured and recorded (swagger-ui assets are the bulk of
      the saving).
- [ ] `cargo tree` no longer lists utoipa* (confirms transitive deps dropped too).

## Technical Notes

- The handcrafted spec at `src/api/routes/openapi.rs` is intentional and stays —
  this ticket only removes the unused codegen/UI crates, not the endpoint.
- If a future ticket wants live Swagger UI, that's a separate decision; today it
  isn't wired up at all.

## Dependencies

- None.
