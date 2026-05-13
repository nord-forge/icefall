# IF-155: Rust performance audit & hot-path optimization

**Phase:** 23 — Rust Quality & Performance
**Priority:** High
**Estimate:** L

## Description

Run the `rust-performance` skill across the codebase to identify and fix allocation hotspots, async bottlenecks, query inefficiencies, and unnecessary clones. Focus on the hot paths: API request handlers, deploy pipeline, SSE streaming, and database queries.

## Acceptance Criteria

### Allocation Audit

- [ ] Audit all API route handlers for unnecessary `String` allocations — use `&str` where possible
- [ ] Replace `format!()` with const strings or `Cow<str>` on hot paths
- [ ] Find `clone()` calls on large types that could use `Arc` or borrows instead
- [ ] Identify `collect::<Vec<_>>()` followed by iteration — replace with iterator chains
- [ ] Check `serde_json::Value` usage — replace with typed deserialization where the schema is known
- [ ] Add `with_capacity()` hints to `Vec` and `HashMap` where the size is known or estimable

### Async Audit

- [ ] Find sequential `.await` calls that could run concurrently via `tokio::join!`
- [ ] Check for blocking operations in async context (file I/O, DNS lookups, heavy computation)
- [ ] Verify `tokio::spawn` calls track their `JoinHandle` for graceful shutdown
- [ ] Audit channel usage — replace `unbounded_channel` with bounded where backpressure is appropriate
- [ ] Check connection pool sizes match expected concurrent load

### Database Query Audit

- [ ] Identify N+1 query patterns (loops with individual queries)
- [ ] Check for missing indexes on columns used in WHERE/JOIN clauses
- [ ] Find `SELECT *` queries that could select specific columns
- [ ] Verify pagination on list endpoints that could return unbounded results
- [ ] Check transaction scopes — are any holding locks longer than needed?
- [ ] Ensure all queries use parameterized values (no string interpolation)

### Deploy Pipeline

- [ ] Profile the deploy hot path: webhook receive → build trigger → image build → container swap
- [ ] Identify the slowest steps and whether any can run concurrently
- [ ] Check image layer caching effectiveness
- [ ] Audit the blue-green swap for unnecessary container operations

### Benchmarks

- [ ] Add criterion benchmarks for:
  - Database query round-trip (create + read + update + delete cycle)
  - Request handler throughput (mock request → response)
  - Env var encryption/decryption round-trip
  - JSON serialization of common response types
- [ ] Document baseline numbers in `benches/README.md`
- [ ] Benchmarks runnable via `cargo bench`

### Fixes

- [ ] Fix all critical and important findings from the audit
- [ ] Each fix is a separate commit with before/after context
- [ ] No regressions in `cargo test`

## Technical Notes

- Use the `rust-performance` skill to run the audit
- For benchmarks, add `criterion` as a dev dependency
- SQLite-specific: check that WAL mode is enabled (it is), and that `PRAGMA journal_size_limit` is set to prevent unbounded WAL growth
- The `sysinfo` crate (used for server stats) does blocking I/O — verify it's called from a blocking task, not the async runtime

## Out of Scope

- Architectural changes (different database, different async runtime)
- Micro-optimizations on cold paths (CLI commands, one-time setup)
- Frontend performance (separate concern)

## Dependencies

- IF-153 (sqlite.rs split — easier to audit when files are smaller)
