# IF-219: Unmanaged container visibility

**Phase:** 24 — Feature Parity
**Priority:** Low
**Estimate:** S

## Description

Show containers running on a server that are NOT managed by Icefall. Users often have other workloads (monitoring agents, VPN clients, legacy apps) running alongside Icefall-managed apps. Seeing all containers in one place gives a complete picture of server resource usage and helps diagnose conflicts.

## Acceptance Criteria

- [ ] Server detail page → Apps tab: new section "Other Containers" below the Icefall-managed apps
- [ ] Lists all running containers that don't have the Icefall management label
- [ ] Per container: name, image, status, CPU/memory usage, ports, created date
- [ ] Basic actions: start, stop, restart (with confirmation dialog)
- [ ] Visual distinction from managed apps (muted style, "unmanaged" badge)
- [ ] For workers: container list fetched via agent WebSocket protocol
- [ ] Refresh button to re-scan containers
- [ ] API endpoint: `GET /servers/{id}/containers?unmanaged=true`

## Technical Notes

- Filter by absence of the `icefall.managed=true` label (or equivalent label used by the deploy manager)
- Use the existing container runtime abstraction (Docker/Podman via bollard) — `list_containers` with label filter
- Metrics for unmanaged containers come from the same stats stream (IF-127)
- Don't allow delete/remove — only lifecycle actions (start/stop/restart)

## Out of Scope

- "Adopting" unmanaged containers into Icefall management
- Viewing unmanaged container logs (use server terminal IF-213 for that)
- Unmanaged volume/network visibility

## Dependencies

- IF-136 (Servers list page)
- IF-138 (Server detail page)
