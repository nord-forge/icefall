# IF-206: Podman runtime support (opt-in)

**Phase:** 25 — Parity Gaps
**Priority:** High
**Estimate:** M

## Description

Add Podman as a supported container runtime alongside Docker. bollard already has first-class Podman support via its Docker-compatible API socket. The implementation is primarily config, install script detection, and validation — not a code rewrite.

## Acceptance Criteria

### Config
- [ ] New `runtime` field in `config.toml`: `"docker"` (default) or `"podman"`
- [ ] `container_socket` field (replaces `docker_socket`): auto-set based on runtime
  - Docker: `/var/run/docker.sock`
  - Podman: `/run/podman/podman.sock`
- [ ] Backward compatible: `docker_socket` still works as an alias
- [ ] Agent config: same `runtime` / `container_socket` pattern

### Install Script
- [ ] Auto-detect existing runtime:
  1. Check for running Docker (`docker info`)
  2. Check for running Podman (`podman info`)
  3. If both: prefer whichever is running
  4. If neither: install Docker (default)
- [ ] Podman path: ensure `podman.socket` systemd unit is enabled (exposes API socket)
- [ ] Podman version check: require >= 4.0 (for netavark networking)
- [ ] Write detected runtime + socket path to `config.toml`

### Validation
- [ ] Startup check: verify the configured socket is reachable (`ping` API call)
- [ ] If socket unreachable: clear error message with runtime-specific fix instructions
- [ ] Log the detected runtime and version on startup

### Podman-Specific Handling
- [ ] Stats CPU calculation: detect Podman runtime, handle `system_cpu_usage = 0` edge case
- [ ] Compose commands: use `podman compose` instead of `docker compose` when runtime is Podman
- [ ] Image cleanup: use same bollard prune APIs (compatible on both)
- [ ] Network creation: ensure named networks are created (Podman doesn't auto-resolve on default network)

### CI
- [ ] GitHub Actions matrix: `runtime: [docker, podman]`
- [ ] Smoke test on Podman: container lifecycle, image build, network DNS, exec, stats, volume mount
- [ ] Terminal/exec test specifically (highest risk area for Podman compat)

### Dashboard
- [ ] Settings page: display detected runtime and version
- [ ] Server detail page: show runtime per server (Docker/Podman)

## Technical Notes

- bollard auto-discovers Podman sockets in this order: `$DOCKER_HOST`, `$XDG_RUNTIME_DIR/podman/podman.sock`, `/run/podman/podman.sock`, `/var/run/docker.sock`
- Icefall already uses `Docker::connect_with_socket()` — just pass the Podman socket path
- Only support rootful Podman initially — rootless has networking + volume + SYS_ADMIN complications
- One runtime per server — don't support both on the same machine

## Out of Scope

- Rootless Podman support (future enhancement)
- Podman pods (grouping concept — not needed when Icefall manages containers directly)
- Quadlet/systemd unit generation (future enhancement, nice-to-have)
- containerd direct support

## Dependencies

- IF-205 (Container runtime research — complete, recommendation: proceed)
