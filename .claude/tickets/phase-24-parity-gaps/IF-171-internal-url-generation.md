# IF-171: Internal URL generation for service-to-service communication

**Phase:** 24 — Parity Gaps
**Priority:** Medium
**Estimate:** S

## Description

Auto-generate internal hostnames for containers on the same Docker network so services can communicate without public DNS. Each app and database gets a predictable internal URL based on its name and the Icefall network.

## Acceptance Criteria

- [ ] Every container gets an internal hostname: `{app-slug}.icefall.internal` (e.g., `my-api.icefall.internal`)
- [ ] Hostname set via Docker `--hostname` and `--network-alias` on the icefall bridge network
- [ ] Database containers: `{db-name}.icefall.internal` with the database port
- [ ] App overview tab: "Internal URL" field displaying the hostname + port (e.g., `http://my-api.icefall.internal:3000`)
- [ ] Database detail page: "Internal URL" field (e.g., `postgres://user:pass@my-db.icefall.internal:5432/app`)
- [ ] Copy button on internal URLs
- [ ] Internal URLs usable as env var values in other apps (e.g., `DATABASE_URL=postgres://...@my-db.icefall.internal:5432/db`)
- [ ] Compose stacks: services within a stack use their Compose service names on the stack's isolated network; the internal URL is for cross-stack communication

## Technical Notes

- Docker's built-in DNS resolution on bridge networks handles this — containers on the same network can resolve each other by hostname
- Set `hostname` and `network_aliases` in bollard's `ContainerCreateOpts`
- The `.icefall.internal` suffix prevents collisions with real DNS names
- For multi-server: internal URLs only work between containers on the same server (same Docker network)

## Dependencies

- IF-004 (Docker Engine client)
- IF-073 (Docker Compose — for cross-stack context)
