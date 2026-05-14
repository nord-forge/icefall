# IF-154: Split remaining large Rust files into sub-modules

**Phase:** 23 — Rust Quality & Performance
**Priority:** Medium
**Estimate:** M

## Description

After the sqlite.rs split (IF-153), several other Rust files exceed 500 lines and would benefit from modular decomposition. This ticket covers splitting the remaining large files into focused sub-modules.

## Files to Split

| File | Lines | Proposed Split |
|------|-------|---------------|
| `src/deploy/compose.rs` | 750 | `compose/mod.rs` (parse + orchestrate), `compose/parser.rs` (YAML parsing + variable interpolation), `compose/network.rs` (bridge network setup), `compose/ordering.rs` (depends_on topological sort) |
| `src/api/routes/oauth.rs` | 726 | `oauth/mod.rs` (router mount), `oauth/github.rs` (GitHub PKCE flow), `oauth/google.rs` (Google PKCE flow), `oauth/linking.rs` (account linking logic) |
| `src/cli/commands/migrate.rs` | 712 | `migrate/mod.rs` (CLI args), `migrate/export.rs` (tar.gz creation), `migrate/import.rs` (restore + validation) |
| `src/api/routes/servers.rs` | 698 | `servers/mod.rs` (router), `servers/crud.rs` (CRUD endpoints), `servers/agent.rs` (enrollment + WebSocket), `servers/metrics.rs` (metric endpoints) |
| `src/deploy/manager.rs` | 694 | `manager/mod.rs` (DeployManager struct + deploy entry point), `manager/local.rs` (local container deploy logic), `manager/remote.rs` (remote deploy via agent), `manager/blue_green.rs` (blue-green swap logic) |
| `src/api/routes/update.rs` | 657 | `update/mod.rs` (router), `update/check.rs` (version check + discovery), `update/apply.rs` (trigger + status), `update/settings.rs` (auto-update config) |
| `src/api/routes/apps.rs` | 598 | `apps/mod.rs` (router), `apps/crud.rs` (create/read/update/delete), `apps/actions.rs` (start/stop/restart/migrate) |
| `src/api/routes/databases.rs` | 554 | `databases/mod.rs` (router), `databases/crud.rs` (provisioning + CRUD), `databases/backups.rs` (backup endpoints), `databases/browser.rs` (query browser) — or merge `db_browser.rs` into this |
| `src/db/models.rs` | 515 | `models/mod.rs` (re-exports), `models/app.rs`, `models/deploy.rs`, `models/user.rs`, `models/server.rs`, `models/settings.rs` — group by domain |
| `src/build/orchestrator.rs` | 510 | `orchestrator/mod.rs` (pipeline entry), `orchestrator/clone.rs` (git clone), `orchestrator/detect.rs` (framework detection dispatch), `orchestrator/build.rs` (container image build) |
| `src/update/apply.rs` | 505 | `apply/mod.rs` (update flow), `apply/download.rs` (fetch + verify), `apply/swap.rs` (atomic binary replacement + migrations) |

## Acceptance Criteria

- [ ] Each file above 500 lines is converted to a directory module with sub-files under 250 lines
- [ ] No public API changes — all exports remain the same
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt -- --check` clean
- [ ] No changes to function signatures or behavior

## Technical Notes

- Work through these in dependency order — files that import from each other should be split together
- The `api/routes/*.rs` files are good candidates for route-group sub-modules since Axum routers compose naturally
- `models.rs` split is optional but recommended — 515 lines of struct definitions is manageable but getting close to the threshold
- Each split should be a separate commit for easy review and revert

## Out of Scope

- Changing function signatures or logic
- Performance optimizations (IF-155)
- Files under 500 lines

## Dependencies

- IF-153 (sqlite.rs split — do this first as a template for the pattern)
