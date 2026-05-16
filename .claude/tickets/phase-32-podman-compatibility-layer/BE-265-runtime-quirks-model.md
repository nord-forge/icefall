# BE-265: RuntimeQuirks model and detection

**Phase:** 32
**Priority:** Critical
**Size:** M
**Dependencies:** None

## Description

Introduce a `RuntimeQuirks` value that captures every way the active container
runtime diverges from baseline Docker behavior. It is resolved once when the
`DockerClient` connects, and threaded through the client so divergence points
branch on data rather than hardcoded assumptions.

## Changes

- New `RuntimeQuirks` struct (in `src/docker/`):
  - `runtime: ContainerRuntime` — Docker | Podman
  - `rootless: bool` — true for rootless Podman
  - `host_bind_ip: String` — `0.0.0.0` for Docker / rootful Podman, `127.0.0.1`
    for rootless Podman
  - `supports_cgroup_limits: bool` — whether `cpu_shares` / `memory` are honored
  - `dns_backend: DnsBackend` — `BuiltIn | Netavark | Unknown`
  - `min_unprivileged_port: u16` — 0 for rootful, 1024 for rootless
- Detection logic: call `podman info` / Docker `version`+`info` at connect time;
  determine rootless from socket path ownership and `info.host.security.rootless`.
- `DockerClient` holds a `RuntimeQuirks`; expose a getter.
- `RuntimeQuirks::docker_default()` for the plain-Docker case.

## Acceptance Criteria

- Given a Docker socket, when the client connects, then `runtime = Docker`,
  `rootless = false`, `host_bind_ip = "0.0.0.0"`.
- Given a rootless Podman socket, when the client connects, then `rootless =
  true` and `host_bind_ip = "127.0.0.1"`.
- Given a rootful Podman socket, when the client connects, then `runtime =
  Podman`, `rootless = false`.
- `RuntimeQuirks` is cloneable and cheap to pass around.

## Out of Scope

Acting on the quirks — that is BE-267/268/269. This ticket only detects and
models them.
