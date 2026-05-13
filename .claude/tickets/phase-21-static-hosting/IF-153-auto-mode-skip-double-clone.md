# IF-153: Eliminate double clone in auto deploy mode

**Phase:** 21 — Static Hosting Expansion
**Priority:** Medium
**Estimate:** S
**Dependencies:** None

## Description

In `src/api/routes/deploys.rs`, when `deploy_mode == "auto"`, the code clones the repo to detect the framework, deletes the clone, then the chosen pipeline (native or container) clones again. This doubles the git clone time for every auto-mode deploy.

Refactor so the initial clone is reused by the chosen pipeline instead of being discarded.

## Acceptance Criteria

- [ ] Auto-mode deploy clones once, passes the work directory to the chosen pipeline
- [ ] `NativeDeployer::deploy()` accepts an optional pre-cloned directory (skips its own clone step)
- [ ] `BuildOrchestrator::build()` accepts an optional pre-cloned directory (skips its own clone step)
- [ ] No behavior change for explicit `deploy_mode: "native"` or `deploy_mode: "container"` — they still clone themselves
- [ ] Deploy logs still show the clone step (just note it was reused)

### Tests
- [ ] Auto-mode deploy completes successfully with single clone
- [ ] Explicit native mode still clones independently
- [ ] Explicit container mode still clones independently

## Out of Scope

- Caching clones across deploys (shallow clone cache)

## Files to Modify

- `src/api/routes/deploys.rs` — pass work_dir to chosen pipeline
- `src/deploy/native.rs` — accept optional pre-cloned dir
- `src/build/orchestrator.rs` — accept optional pre-cloned dir
