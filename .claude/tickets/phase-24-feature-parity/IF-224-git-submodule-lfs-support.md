# IF-224: Git submodule and LFS support

**Phase:** 24 — Feature Parity
**Priority:** Low
**Estimate:** S

## Description

Add toggles for git submodule initialization and Git LFS checkout during the clone phase of the build pipeline. Some monorepos use submodules for shared libraries, and projects with large binary assets (game assets, ML models, design files) use Git LFS. Without these, builds fail silently or produce incomplete artifacts.

## Acceptance Criteria

- [ ] Per-app toggle in advanced settings: "Enable Git Submodules" (default: off)
- [ ] When enabled: run `git submodule update --init --recursive` after clone
- [ ] Per-app toggle in advanced settings: "Enable Git LFS" (default: off)
- [ ] When enabled: run `git lfs install && git lfs pull` after clone
- [ ] Per-app toggle: "Shallow Clone" (default: on) — `git clone --depth 1` for faster clones
- [ ] When submodules are enabled with shallow clone: `--shallow-submodules` flag
- [ ] Build log shows submodule/LFS steps with output
- [ ] If submodule init or LFS pull fails: build fails with clear error message
- [ ] API: `PUT /apps/{id}` accepts `git_submodules_enabled`, `git_lfs_enabled`, `git_shallow_clone`
- [ ] For multi-server: settings passed to agent build pipeline

## Technical Notes

- These are flags on the git clone step in `src/deploy/` — minimal code change
- LFS requires `git-lfs` to be installed on the build server / agent — detect and warn if missing
- Shallow clone is already the default for performance; making it toggleable handles edge cases where full history is needed (e.g., version stamping from git describe)

## Out of Scope

- Sparse checkout (partial clone of specific paths — covered partially by IF-160 base directory)
- Git credential management for private submodules (use deploy keys)
- LFS bandwidth monitoring

## Dependencies

- IF-010 (Image build orchestrator — clone step)
- IF-132 (Agent build pipeline — for multi-server)
