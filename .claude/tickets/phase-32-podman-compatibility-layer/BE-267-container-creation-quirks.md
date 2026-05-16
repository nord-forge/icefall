# BE-267: Quirk-aware container creation

**Phase:** 32
**Priority:** Critical
**Size:** M
**Dependencies:** BE-265

## Description

`create_container()` in `src/docker/containers.rs` hardcodes Docker-API
assumptions that break or silently misbehave on Podman, especially rootless.
Branch these on `RuntimeQuirks` so a single code path works for both runtimes.

## Changes to `src/docker/containers.rs`

- **Host-port binding:** replace the hardcoded `host_ip: "0.0.0.0"` with
  `quirks.host_bind_ip`. For rootless Podman this becomes `127.0.0.1` (Caddy
  runs on the same host and proxies to it, so loopback is sufficient).
- **Privileged ports:** if `quirks.rootless` and a requested `host_port` is
  below `quirks.min_unprivileged_port`, return a clear `DockerError` explaining
  rootless cannot bind it — rather than a cryptic runtime failure.
- **Restart policy:** when `quirks.runtime == Podman`, document/handle that
  `unless-stopped` and `always` are realized via systemd; keep the mapping but
  log a warning if rootless `always` is requested without lingering.
- **cgroup limits:** if `!quirks.supports_cgroup_limits`, still pass
  `memory` / `cpu_shares` but emit a one-time warning that limits may be
  ignored, so operators are not surprised.

## Acceptance Criteria

- Given rootless Podman, when a container is created, then host ports bind to
  `127.0.0.1` and the container is reachable by Caddy.
- Given rootless Podman and a requested host port < 1024, when create is called,
  then a clear error is returned before hitting the runtime.
- Given Docker, when a container is created, then behavior is byte-for-byte
  identical to today (no regression).
- Given rootless Podman without cgroup delegation, when resource limits are
  set, then creation still succeeds and a warning is logged.

## Out of Scope

Networking/DNS (BE-268), image transfer (BE-269).
