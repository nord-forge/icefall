# BE-266: Rootless Podman socket detection

**Phase:** 32
**Priority:** Critical
**Size:** S
**Dependencies:** None

## Description

`detect_socket()` only probes the **rootful** runtime socket paths. Rootless
Podman exposes its API socket under `$XDG_RUNTIME_DIR`, owned by the invoking
user — so a rootless setup is currently missed and the daemon falls back to a
Docker socket that does not exist.

## Changes

- Extend `detect_socket()` (`config/defaults.rs`) to also probe, in order:
  1. `$XDG_RUNTIME_DIR/podman/podman.sock` (rootless Podman)
  2. `/run/user/$(id -u)/podman/podman.sock` (fallback if `XDG_RUNTIME_DIR` unset)
  3. `/run/podman/podman.sock`, `/var/run/podman/podman.sock` (rootful Podman)
  4. `/var/run/docker.sock` (Docker)
- `detect_runtime_from_socket()` already keys off the `podman` substring — keep
  that working for the rootless paths.
- Install script: when `--runtime=podman` and running as a non-root user (or
  rootless explicitly requested), detect and write the rootless socket path
  into `config.toml` instead of the rootful one.
- Document the rootless socket path in the runtime config docs.

## Acceptance Criteria

- Given only a rootless Podman socket exists, when `detect_socket()` runs, then
  it returns the `$XDG_RUNTIME_DIR` path.
- Given both rootless Podman and Docker sockets exist, when `detect_socket()`
  runs, then the Podman socket is preferred (consistent with current ordering).
- Given a rootless install, when `config.toml` is generated, then
  `container_socket` points at the rootless path.

## Out of Scope

Enabling/starting the rootless socket unit (`systemctl --user enable
podman.socket`) — that is an install-script / docs concern, covered briefly in
docs but not automated here.
