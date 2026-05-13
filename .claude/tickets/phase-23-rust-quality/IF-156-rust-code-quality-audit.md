# IF-156: Rust code quality audit — safety, error handling, idiomatic patterns

**Phase:** 23 — Rust Quality & Performance
**Priority:** High
**Estimate:** M

## Description

Run the `rust-review` skill across the entire Rust codebase to catch safety issues, non-idiomatic patterns, error handling gaps, and potential panics that Clippy doesn't flag. This is a comprehensive quality pass that goes beyond linting.

## Acceptance Criteria

### Safety Audit

- [ ] Find all `unwrap()` and `expect()` in non-test code — categorize as:
  - **Safe**: provably can't fail (e.g., regex compilation of a constant)
  - **Risky**: could fail under edge conditions — replace with `?` or proper error handling
  - **Dangerous**: in a request handler or background task — must fix
- [ ] Audit all uses of `unsafe` (if any exist)
- [ ] Check for unchecked indexing (`vec[i]`) that could panic — use `.get()` with proper handling
- [ ] Verify all user input is validated before use (path traversal, injection, overflow)
- [ ] Check that secrets are not logged (grep for `tracing::info!`/`debug!` near password/token/key variables)
- [ ] Verify `zeroize` is used on sensitive data after use (env var decryption, passwords)

### Error Handling Audit

- [ ] Find `map_err(|_| ...)` patterns that discard error context — preserve the original error
- [ ] Check error variant coverage — are all `DbError` / `DeployError` variants meaningful and distinct?
- [ ] Verify error responses don't leak internal details to API consumers (stack traces, file paths, SQL)
- [ ] Check that all `Result` returns in handlers map to appropriate HTTP status codes
- [ ] Find any `.ok()` calls that silently swallow errors

### Idiomatic Rust

- [ ] Functions with 5+ parameters — should these use a builder or config struct?
- [ ] `String` parameters that could be `impl Into<String>` or `&str`
- [ ] Missing `#[derive(Debug)]` on types that might need debugging
- [ ] `async fn` that never awaits — should be sync
- [ ] Redundant type annotations that the compiler can infer
- [ ] `pub` visibility on items that should be `pub(crate)` or private
- [ ] Unused `allow` attributes from development that should be removed

### Consistency

- [ ] Error type usage is consistent (same error type for same module)
- [ ] Naming conventions: `new()` for constructors, `into_*()` for ownership transfer, `as_*()` for cheap conversions
- [ ] Response shapes follow the same pattern across all API endpoints
- [ ] All async trait methods use `#[async_trait]` consistently

### Fixes

- [ ] Fix all critical and important findings
- [ ] Each fix is a separate commit
- [ ] No regressions in `cargo test`
- [ ] `cargo clippy -- -D warnings` clean after all fixes

## Technical Notes

- Use the `rust-review` skill to run the audit
- Start with request handlers (`src/api/routes/`) — these are the public-facing code with the highest risk
- Then audit the deploy pipeline (`src/deploy/`) — failures here affect users directly
- Finally audit background tasks (`src/monitoring/`, `src/update/`) — failures here are harder to notice
- The agent crate (`agent/`) should be audited separately since it runs on worker servers

## Out of Scope

- Performance optimizations (IF-155)
- Module splitting (IF-153, IF-154)
- Adding new tests (separate concern, though audit findings may suggest tests)
- Frontend/dashboard code

## Dependencies

- None (can run in parallel with other tickets)
