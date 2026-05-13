# IF-203: Troubleshooting guide and FAQ

**Phase:** 27 — Comprehensive Docs
**Priority:** High
**Estimate:** M

## Description

Write a comprehensive troubleshooting section covering every common issue a user might encounter. Organized by symptom ("my deploy failed", "my app is returning 502") with diagnostic steps and solutions. Plus an FAQ for questions that don't fit troubleshooting.

## Pages to Create

### `troubleshooting/deploy-failures.mdx`
- [ ] Build fails: missing dependencies, wrong Node version, Docker build errors
- [ ] Deploy fails: container crashes on start, port mismatch, health check timeout
- [ ] Rollback fails: previous image pruned, container won't start
- [ ] Webhook not triggering: wrong URL, secret mismatch, branch filter
- [ ] Slow builds: cache invalidation, large dependencies, no multi-stage

### `troubleshooting/networking.mdx`
- [ ] 502 Bad Gateway: container not running, wrong port, Caddy misconfigured
- [ ] SSL certificate errors: DNS not propagated, rate limit, CAA records
- [ ] DNS not resolving: wrong record type, TTL, propagation time
- [ ] Can't reach app from internet: firewall, ports not open, tunnel not configured
- [ ] WebSocket not working: Caddy config, proxy timeout

### `troubleshooting/databases.mdx`
- [ ] Can't connect to database: wrong credentials, network, firewall
- [ ] Database backup fails: disk full, permissions, timeout
- [ ] Database slow: missing indexes, connection pool exhausted
- [ ] Data loss on redeploy: volume not configured

### `troubleshooting/multi-server.mdx`
- [ ] Agent won't connect: firewall, token expired, DNS resolution
- [ ] Server shows offline: heartbeat timeout, network issues
- [ ] Deploy to remote server fails: agent error, Docker not running
- [ ] Migration fails: volume loss, server not connected

### `troubleshooting/performance.mdx`
- [ ] App using too much memory: no resource limits, memory leak, swap
- [ ] High CPU usage: event loop blocking, computation in request handler
- [ ] Disk full: Docker images, logs, database dumps
- [ ] Slow dashboard: too many apps polling, SSE connections

### `troubleshooting/auth.mdx`
- [ ] Can't log in: wrong password, 2FA issues, locked account
- [ ] OAuth callback error: redirect URI mismatch, provider config
- [ ] API token not working: expired, wrong scope, revoked
- [ ] Lost 2FA device: backup codes, admin reset

### `troubleshooting/podman.mdx`
- [ ] Podman socket not active: `systemctl enable --now podman.socket`
- [ ] Podman version too old: minimum 4.0 required, how to upgrade
- [ ] Container networking: containers can't reach each other (need named network, not default)
- [ ] Volume permission errors: UID mapping in rootless mode, `:Z` / `:U` suffixes
- [ ] Terminal/exec not working: known attach/stream compat issues, workarounds
- [ ] CPU stats showing 0%: Podman stats edge case, expected behavior in rootless
- [ ] Image build differences: Buildah vs BuildKit behavior
- [ ] "Connection refused" on Podman socket: socket service not started, wrong path
- [ ] Switching from Docker to Podman: migration steps, what to watch for
- [ ] Compose stack issues on Podman: `podman compose` vs `podman-compose` differences

### `faq.mdx`
- [ ] How is Icefall different from Coolify/Dokku/CapRover?
- [ ] Can I run Icefall on ARM servers?
- [ ] How much RAM does Icefall need?
- [ ] Can I use Icefall with an existing Docker setup?
- [ ] Can I use Podman instead of Docker?
- [ ] What's the difference between Docker and Podman in Icefall?
- [ ] Does Icefall support Kubernetes?
- [ ] How do I migrate from Coolify/Dokku to Icefall?
- [ ] Can I use my own reverse proxy instead of Caddy?
- [ ] How do backups work? Where are they stored?
- [ ] Is Icefall production-ready?
- [ ] How do I contribute?
- [ ] Can I switch from Docker to Podman on an existing installation?

## Standards

- [ ] Symptom-first organization: user searches for what they SEE, not what's wrong
- [ ] Each issue: symptoms, diagnostic commands, solution, prevention
- [ ] Include actual error messages users will see (searchable)
- [ ] Link to relevant concept docs for deeper understanding
- [ ] All diagnostic commands shown for both Docker and Podman

## Dependencies

- IF-047 (Documentation site)
- IF-206 (Podman runtime support)
