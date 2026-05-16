# BE-268: Podman networking & DNS compatibility

**Phase:** 32
**Priority:** High
**Size:** M
**Dependencies:** BE-265

## Description

Icefall creates a per-app Docker network and relies on container DNS so
containers can resolve each other by hostname (e.g. the
`{app}-i{n}.icefall.internal` names used by Phase 31 multi-instance). Docker
has built-in DNS on user-defined networks; Podman needs `netavark` +
`aardvark-dns`, and on older or misconfigured hosts inter-container name
resolution may not work.

## Changes

- During `RuntimeQuirks` detection (BE-265), determine the DNS backend:
  `netavark` (modern Podman) vs unknown/legacy.
- `create_network()` (`src/docker/networks.rs`): on Podman, ensure the network
  is created with DNS enabled (netavark provides this by default; verify the
  options passed via `bollard` request it).
- If `dns_backend` is not DNS-capable, fall back to wiring containers by
  explicit IP / extra_hosts instead of relying on name resolution, or surface a
  clear startup warning telling the operator to install `aardvark-dns`.
- Verify the Phase 31 inter-instance hostname scheme resolves on Podman; if it
  does not, adjust the deploy pipeline to not depend on container DNS for
  multi-instance routing (Caddy already routes by `host:port`, so internal DNS
  may simply be unnecessary — confirm and document).

## Acceptance Criteria

- Given Podman with netavark, when an app network is created, then containers
  on it can resolve each other by name.
- Given Podman without `aardvark-dns`, when the daemon starts, then a clear
  warning is logged with the remediation step.
- Given a multi-instance app on Podman, when deployed, then Caddy routes to all
  instances correctly regardless of internal DNS state.

## Out of Scope

Custom CNI plugin configuration; advanced network topologies.
