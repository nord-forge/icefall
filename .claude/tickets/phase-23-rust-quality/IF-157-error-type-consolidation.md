# IF-157: Consolidate error types and improve error chain

**Phase:** 23 — Rust Quality & Performance
**Priority:** Medium
**Estimate:** M

## Description

The codebase has multiple error enums (`DbError`, `DeployError`, `HandlerError`, etc.) with overlapping variants and inconsistent conversion chains. Audit and consolidate the error types so that every error has a clear origin, context is preserved through the chain, and API consumers get meaningful error responses without internal leaks.

## Current State

Key error types to audit:
- `DbError` — database layer
- `DeployError` — deploy pipeline (has many variants: `ContainerCreate`, `RemoteOp`, `AgentOffline`, `AgentTimeout`, `RemoteBuild`, `EnvelopeEncrypt`, etc.)
- `HandlerError` — agent message handlers
- `ApiError` — HTTP response errors
- Various ad-hoc `String` errors in smaller modules

## Acceptance Criteria

### Audit

- [ ] Map every error type, its variants, and where each variant is constructed
- [ ] Identify duplicate/overlapping variants across types
- [ ] Find places where `String` is used as an error type instead of a proper enum
- [ ] Find `map_err(|e| format!(...))` patterns that lose the original error type
- [ ] Check the full chain from origin → API response for each error path

### Consolidation

- [ ] Each module has at most one error enum
- [ ] Error variants are specific and actionable (not `Other(String)` catch-alls — minimize these)
- [ ] `#[from]` conversions between error types form a clear hierarchy without cycles
- [ ] `thiserror` is used consistently for all error types
- [ ] Error display messages are user-safe (no file paths, SQL, or stack traces)
- [ ] Internal context preserved via `#[source]` for logging

### API Error Mapping

- [ ] Every error variant maps to a specific HTTP status code
- [ ] 4xx errors include a machine-readable `code` field (e.g., `"app_not_found"`, `"deploy_in_progress"`)
- [ ] 5xx errors log the full chain internally but return only a generic message to the client
- [ ] Consistent error response shape across all endpoints:
  ```json
  { "error": { "code": "app_not_found", "message": "Application not found" } }
  ```

### Tests

- [ ] Add tests for error conversion chains (DbError → ApiError mapping)
- [ ] Verify no error response leaks internal paths or SQL
- [ ] `cargo test` green, `cargo clippy` clean

## Technical Notes

- `DeployError` is the most complex — it has ~15 variants and is used across local deploys, remote deploys, and the build pipeline
- Consider a `context()` helper pattern (like `anyhow::Context`) for adding context without losing the original error
- The `HandlerError` in the agent crate should mirror the control plane error pattern for consistency

## Out of Scope

- Changing error behavior (how errors are handled) — only how they're typed and propagated
- Adding retry logic on errors
- Frontend error handling

## Dependencies

- IF-153 (sqlite.rs split — easier to audit error paths in smaller files)
