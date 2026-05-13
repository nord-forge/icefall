# IF-153: Split sqlite.rs into domain-specific modules

**Phase:** 23 — Rust Quality & Performance
**Priority:** High
**Estimate:** M

## Description

`src/db/sqlite.rs` is 2204 lines with 128 functions implementing the entire `Database` trait in a single file. Split it into domain-specific sub-modules under `src/db/sqlite/` while keeping the public API unchanged. Each sub-module implements a subset of the `Database` trait methods using a partial impl pattern or helper functions called from the main impl block.

## Current State

| Domain | Functions | Approx Lines |
|--------|-----------|-------------|
| Projects | 5 | ~100 |
| Apps | 7 | ~180 |
| Environments + Env Vars | 6 | ~130 |
| Deploys | 5 | ~120 |
| Managed Databases | 5 | ~150 |
| Domains | 4 | ~80 |
| Users + Auth | 10 | ~180 |
| Health Checks | 5 | ~120 |
| Notifications | 6 | ~120 |
| Backups | 7 | ~140 |
| Settings | 7 | ~140 |
| API Tokens + Sessions | 8 | ~150 |
| Audit Log | 3 | ~60 |
| Update State | 15 | ~250 |
| Servers + Metrics | 12 | ~230 |
| Misc (connect, vacuum, migrations) | 3 | ~50 |

## Acceptance Criteria

### Module Structure

- [ ] Convert `src/db/sqlite.rs` into `src/db/sqlite/mod.rs` + sub-modules
- [ ] Proposed structure:
  ```
  src/db/sqlite/
    mod.rs          — SqliteDatabase struct, connect(), run_migrations(), re-exports
    projects.rs     — project CRUD
    apps.rs         — app CRUD
    env_vars.rs     — environments + env var operations
    deploys.rs      — deploy CRUD + status updates
    databases.rs    — managed database operations
    domains.rs      — domain CRUD + status
    users.rs        — user CRUD, password, email, TOTP
    health.rs       — health checks + events
    notifications.rs — notification channels + rules
    backups.rs      — backup records + schedules
    settings.rs     — settings key-value store
    tokens.rs       — API tokens + sessions
    audit.rs        — audit log
    updates.rs      — update state + history
    servers.rs      — server CRUD + metrics
  ```
- [ ] Each sub-module file is under 200 lines
- [ ] `mod.rs` contains only the struct definition, `connect()`, `run_migrations()`, `vacuum_into()`, and the `#[async_trait] impl Database for SqliteDatabase` block that delegates to methods defined in sub-modules

### Implementation Pattern

- [ ] Sub-modules define `impl SqliteDatabase` blocks with the actual query logic
- [ ] The main `impl Database for SqliteDatabase` in `mod.rs` calls these methods directly (they're inherent methods on the same type)
- [ ] Sub-modules access `self.pool` and `self.encryptor` through `&self` — no new abstractions needed
- [ ] Helper function `normalize_repo_url` moves to `apps.rs` (only used there)

### No Behavior Change

- [ ] All existing tests pass without modification
- [ ] `cargo test` green
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt -- --check` clean
- [ ] No changes to the `Database` trait in `mod.rs`
- [ ] No changes to `models.rs` or `encryption.rs`
- [ ] The public API (`SqliteDatabase`, `Database` trait) is unchanged — no other files need updating

## Technical Notes

- This is a pure refactor — move code, no logic changes
- The `#[async_trait]` macro on the trait impl must stay in `mod.rs` since that's where the trait is implemented
- Each sub-module needs `use` imports for sqlx, models, and encryption — extract a common prelude if the imports get repetitive
- Test modules at the bottom of `sqlite.rs` should move to their respective sub-modules or a dedicated `tests.rs`

## Out of Scope

- Changing the Database trait interface
- Optimizing query logic (separate ticket IF-155)
- Adding new database methods
- Splitting `models.rs` (515 lines — worth doing but separate concern)

## Dependencies

- None (pure refactor)
