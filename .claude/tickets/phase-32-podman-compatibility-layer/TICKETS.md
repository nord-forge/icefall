# Phase 32: Podman Compatibility Layer

> Priority: v2.0
> Estimated effort: L (1-2 weeks)
> Dependencies: None (builds on the existing `DockerClient` in `src/docker/`)

## Overview

Make Icefall work reliably on Podman — including **rootless** Podman — not just
Docker. Icefall already talks to either runtime through `bollard` over a Unix
socket, because Podman ships a Docker-compatible REST API. But the current
`DockerClient` hardcodes Docker-API assumptions in a handful of places that
break or silently misbehave on Podman, especially rootless.

The goal is **one quirk-aware client**, not a second client: a `RuntimeQuirks`
value resolved once at connection time and threaded through the existing
`DockerClient`, branching only at the ~6 known divergence points. "100%
compatible" in practice means "every known Docker/Podman divergence is detected
and handled, and that is verified by tests."

## Current State

- `ContainerRuntime` enum (`config/mod.rs`) — `Docker | Podman`, with
  `default_socket()` and `compose_command()`. Already exists.
- `DockerClient` (`src/docker/`, ~834 LOC) — connects via `bollard` to a Unix
  socket; `runtime_version()` introspects whether Docker or Podman answered.
- `detect_socket()` (`config/defaults.rs`) — probes **rootful** Podman socket
  paths then Docker; rootless (`$XDG_RUNTIME_DIR/podman/podman.sock`) is missed.
- The install script (Phase 32 prep) now supports `--runtime=docker|podman|auto`
  with an explicit choice + auto-detect fallback.
- **No Podman-specific tests exist.** Compatibility is currently assumed, not
  verified.

## Known divergence points (the actual work)

1. **Rootless host-port binding** — `containers.rs` hardcodes
   `host_ip: "0.0.0.0"`; rootless Podman needs `127.0.0.1` (or omitted) and
   cannot bind ports < 1024.
2. **Rootless socket path** — rootless Podman's socket is under
   `$XDG_RUNTIME_DIR`, owned by the user, not `/run/podman/podman.sock`.
3. **Restart policy semantics** — Podman maps restart policies to systemd;
   `unless-stopped` / `always` behave differently, and rootless `always`
   needs lingering enabled.
4. **cgroup resource limits** — rootless Podman often ignores `cpu_shares` /
   `memory` unless cgroups v2 + delegation is configured.
5. **Container DNS** — Podman needs `aardvark-dns`/`netavark`; the
   `{app}.icefall.internal` inter-instance hostnames may not resolve.
6. **Image transfer (`docker save`/`load`)** — Phase 31's chunked image
   transfer must be verified against Podman's `import_image` behavior.

## Tickets

| ID | Title | Priority | Size | Dependencies | Status |
|---|---|---|---|---|---|
| [BE-265](BE-265-runtime-quirks-model.md) | RuntimeQuirks model and detection | Critical | M | None | Not started |
| [BE-266](BE-266-rootless-socket-detection.md) | Rootless Podman socket detection | Critical | S | None | Not started |
| [BE-267](BE-267-container-creation-quirks.md) | Quirk-aware container creation | Critical | M | BE-265 | Not started |
| [BE-268](BE-268-networking-dns-compat.md) | Podman networking & DNS compatibility | High | M | BE-265 | Not started |
| [BE-269](BE-269-image-transfer-verification.md) | Image transfer Podman verification | High | S | BE-265 | Not started |
| [QA-270](QA-270-podman-integration-tests.md) | Podman integration test matrix | High | M | BE-267, BE-268 | Not started |

## Dependency Graph

```
BE-265 (RuntimeQuirks)
  ├── BE-267 (container creation)
  │     └── QA-270 (integration tests)
  ├── BE-268 (networking & DNS)
  │     └── QA-270
  └── BE-269 (image transfer)
BE-266 (rootless socket)  ──────── independent, feeds detection
```

## Out of Scope

- Podman-specific compose orchestration beyond what `compose_command()` covers.
- Supporting Podman < 4.0 (install script already enforces >= 4.0).
- Kubernetes / `podman play kube` workflows.
- Windows/macOS Podman machine setups (Icefall daemon targets Linux servers).
