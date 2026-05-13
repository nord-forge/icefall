# IF-172: Public port / TCP proxy for database access

**Phase:** 24 — Parity Gaps
**Priority:** Medium
**Estimate:** M

## Description

Allow users to expose database containers (and other TCP services) on a public port for external access. Caddy's Layer 4 (L4) module handles TCP proxying. Users can connect from local tools (pgAdmin, TablePlus, DBeaver) to their managed databases.

## Acceptance Criteria

- [ ] Database detail page: "Public Access" toggle
- [ ] When enabled: allocate a port from a configurable range (default: 10000-10100)
- [ ] Caddy L4 route: TCP proxy from `0.0.0.0:{allocated_port}` to the container's internal port
- [ ] Display the public connection string: `host:allocated_port` with credentials
- [ ] Warning: "This exposes the database to the internet. Use strong credentials and consider IP whitelisting."
- [ ] Optional IP whitelist: comma-separated list of allowed IPs/CIDRs
- [ ] Port allocation tracked in a `public_ports` table: `id`, `resource_type`, `resource_id`, `port`, `created_at`
- [ ] Port released when public access is disabled or the database is deleted
- [ ] Settings page: configurable port range (admin only)
- [ ] Works for any TCP service, not just databases (extensible to apps that need raw TCP)

## Technical Notes

- Caddy L4 module (`github.com/mholt/caddy-l4`) adds TCP/UDP proxying — check if it's included in the Caddy build
- If L4 is not available: fall back to `socat` or a simple TCP proxy container
- Port range: default 10000-10100 gives 100 public ports, far more than most single-server setups need
- For multi-server: ports are allocated per-server (the agent manages Caddy on its server)

## Out of Scope

- UDP proxying
- Automatic TLS for TCP connections (users should use application-level TLS or SSH tunnels)
- Port forwarding through Cloudflare Tunnel (CF tunnels only support HTTP)

## Dependencies

- IF-005 (Caddy admin API client)
- IF-029 (Managed database provisioning)
