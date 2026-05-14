# IF-160: Monorepo support (base directory)

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** S

## Description

Allow users to configure a subdirectory as the build context for monorepo deployments. A `base_directory` field on the app model tells the build pipeline to use that subdirectory as the root for framework detection, Dockerfile generation, and Docker build context.

## Acceptance Criteria

- [ ] `base_directory` optional text field on the `apps` table (nullable, default null = repo root)
- [ ] App creation wizard: optional "Base directory" input in the build settings step (e.g., `apps/frontend`, `packages/api`)
- [ ] App settings tab: editable base directory field
- [ ] Build pipeline: after git clone, `cd` into `base_directory` before framework detection and build
- [ ] Container image build context set to the base directory, not repo root (Docker/Podman)
- [ ] Validation: path must be relative (no leading `/`), no `..` traversal
- [ ] Webhook deploys respect the base directory
- [ ] Preview environments respect the base directory

## Technical Notes

- The `orchestrator.rs` clone step already clones to a temp dir — just append the base_directory as the working subdirectory
- Framework detection (`detect()`) already accepts a path — pass the subdirectory path

## Dependencies

- IF-010 (Image build orchestrator)
