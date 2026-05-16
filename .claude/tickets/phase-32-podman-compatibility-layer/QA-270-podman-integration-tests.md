# QA-270: Podman integration test matrix

**Phase:** 32
**Priority:** High
**Size:** M
**Dependencies:** BE-267, BE-268

## Description

The "Icefall always works on Podman, even rootless" guarantee can only be
*claimed* if it is *tested*. Establish an integration test matrix that
exercises the deploy lifecycle against Docker, rootful Podman, and rootless
Podman.

## Test Matrix

| Scenario | Docker | Rootful Podman | Rootless Podman |
|---|---|---|---|
| Daemon connects, runtime detected | ✓ | ✓ | ✓ |
| `RuntimeQuirks` resolved correctly | ✓ | ✓ | ✓ |
| Single-app deploy: build → run → route | ✓ | ✓ | ✓ |
| Host-port binding reachable by Caddy | ✓ | ✓ | ✓ |
| Multi-instance deploy (Phase 31) | ✓ | ✓ | ✓ |
| Image transfer between servers | ✓ | ✓ | ✓ |
| Container DNS / inter-instance routing | ✓ | ✓ | ✓ |
| Resource limits applied or warned | ✓ | ✓ | ✓ |

## Changes

- A test module exercising `RuntimeQuirks` detection against each runtime.
- CI job(s) that install rootful and rootless Podman and run the deploy
  lifecycle end to end. If full CI coverage is infeasible, provide a documented
  manual verification script + checklist that a maintainer runs before release.
- Tests must clearly skip (not fail) when a given runtime is unavailable in the
  environment, so the suite stays green locally.

## Acceptance Criteria

- All matrix scenarios pass on Docker and rootful Podman in CI.
- Rootless Podman scenarios pass in CI or via the documented manual checklist.
- A runtime not present in the environment causes a skip, not a failure.

## Out of Scope

Performance benchmarking across runtimes; cross-distro matrix beyond the
distros the install script already supports.
