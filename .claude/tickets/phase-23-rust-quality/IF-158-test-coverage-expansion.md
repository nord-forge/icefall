# IF-158: Expand Rust test coverage for critical paths

**Phase:** 23 — Rust Quality & Performance
**Priority:** Medium
**Estimate:** L

## Description

The codebase has ~90 tests but coverage is uneven — some modules have thorough tests while others (especially the deploy pipeline and API routes) have minimal or no test coverage. Add targeted tests for the highest-risk paths: deploy pipeline, API request handlers, error handling, and the agent protocol.

## Current Test Inventory

Run `cargo test -- --list` to get the full count. Key gaps based on code inspection:

| Module | Estimated Coverage | Risk |
|--------|-------------------|------|
| `src/db/sqlite.rs` | Moderate (basic CRUD) | Medium |
| `src/deploy/manager.rs` | Low | **High** |
| `src/deploy/compose.rs` | Low | High |
| `src/deploy/envelope.rs` | Good (round-trip test) | Low |
| `src/api/routes/*` | Low-None | **High** |
| `src/build/orchestrator.rs` | Low | High |
| `src/monitoring/*` | Low | Medium |
| `src/update/*` | Low | Medium |
| `agent/src/handlers/*` | Low | High |

## Acceptance Criteria

### Deploy Pipeline Tests

- [ ] Unit test: `DeployManager::deploy` with a mock container runtime client — verify the blue-green sequence
- [ ] Unit test: rollback triggers when health check fails
- [ ] Unit test: Compose parser handles valid multi-service files
- [ ] Unit test: Compose parser rejects invalid/malformed files gracefully
- [ ] Unit test: env var resolution follows the correct precedence

### API Route Tests

- [ ] Integration test: app CRUD cycle (create → get → update → delete) via Axum test client
- [ ] Integration test: deploy trigger returns correct status codes
- [ ] Integration test: auth middleware rejects unauthenticated requests
- [ ] Integration test: role enforcement (viewer can't deploy, deployer can't manage users)
- [ ] Integration test: error responses match the expected shape

### Agent Protocol Tests

- [ ] Unit test: agent message serialization/deserialization round-trip
- [ ] Unit test: heartbeat timeout detection
- [ ] Unit test: request/response matching via request ID

### Database Tests

- [ ] Test: concurrent writes don't corrupt data (WAL mode validation)
- [ ] Test: encrypted env vars round-trip correctly
- [ ] Test: cascade deletes work (delete project → apps nullified)
- [ ] Test: migration runs cleanly on a fresh database

### Edge Cases

- [ ] Test: deploy with no env vars
- [ ] Test: deploy with 100+ env vars (bulk)
- [ ] Test: app name with special characters
- [ ] Test: domain with unicode (IDN)
- [ ] Test: very long log lines (>10KB) don't panic

### Infrastructure

- [ ] Add `cargo-llvm-cov` or `cargo-tarpaulin` to CI for coverage reporting
- [ ] Coverage threshold: 60% line coverage for `src/` (baseline to improve from)
- [ ] Coverage report generated on each CI run, linked in PR checks

## Technical Notes

- Use `axum::test::TestClient` for API route tests — it runs the full middleware stack
- For database tests, use an in-memory SQLite database (`sqlite::memory:`)
- Mock the container runtime client using a trait + mock implementation rather than hitting a real Docker/Podman daemon
- The agent tests can use `tokio::sync::mpsc` to simulate the WebSocket connection
- Keep integration tests in `tests/` directory, unit tests next to the code

## Out of Scope

- Frontend/dashboard tests
- E2E tests that require a running container runtime daemon (Docker/Podman)
- Load/stress testing
- Fuzzing (valuable but separate initiative)

## Dependencies

- IF-153 (sqlite.rs split — easier to write focused tests for smaller modules)
- IF-157 (error consolidation — test the final error types, not the pre-consolidation ones)
