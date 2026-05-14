# IF-152: Automated container cleanup

**Phase:** 22 — Expansion (v1.2)
**Priority:** Medium
**Estimate:** S

## Description

Prevent disk exhaustion on long-running servers by automatically cleaning up unused container resources (Docker or Podman). A background task runs on a configurable schedule and removes dangling images, stopped containers, unused volumes, and unused networks. Cleanup is disk-threshold-aware and skips execution during active deployments to prevent race conditions.

## Acceptance Criteria

### Cleanup Logic

- [ ] Cleanup targets (each independently toggleable):
  - **Dangling images**: images not tagged and not referenced by any container (image prune)
  - **Unused images**: images not referenced by any running container (more aggressive, opt-in) (image prune -a)
  - **Stopped containers**: containers in `exited` or `dead` state older than a configurable age (default: 24 hours)
  - **Unused volumes**: volumes not referenced by any container (volume prune) — **disabled by default** (data loss risk)
  - **Unused networks**: networks not connected to any container, excluding `bridge`, `host`, `none`, and `icefall-*` networks
- [ ] Never clean up:
  - Running containers
  - Containers/images/volumes with label `icefall.managed=true` that are actively in use
  - The `icefall-cloudflared` container or its image
  - Images used by the current blue or green deployment of any app
  - Database volumes (volumes with label `icefall.database=true`)
- [ ] Deployment lock: cleanup task checks for active deployments before running. If any deploy is in progress, skip this cycle and retry next schedule.
- [ ] Each cleanup run logs: resources found, resources removed, disk space reclaimed

### Scheduling

- [ ] Background task running on a cron schedule (default: daily at 03:00 server time)
- [ ] Configurable schedule via Settings page (cron expression or preset: daily/weekly/custom)
- [ ] Manual trigger: "Run Cleanup Now" button in Settings
- [ ] Minimum interval: 1 hour (prevent accidental frequent runs)

### Disk Threshold

- [ ] Configurable disk usage threshold (default: 80%)
- [ ] When disk usage exceeds threshold: cleanup runs immediately regardless of schedule (checked every 10 minutes)
- [ ] When disk usage exceeds 90%: aggressive cleanup (unused images enabled even if normally off), plus a notification event (`system.disk_warning`)
- [ ] Disk usage check uses the container runtime data directory partition (`/var/lib/docker` for Docker, `~/.local/share/containers` for rootless Podman, `/var/lib/containers` for rootful Podman)

### Settings UI

- [ ] New "Container Cleanup" section in Settings page
- [ ] Controls:
  - Enable/disable automatic cleanup (toggle)
  - Schedule selector: daily at time / weekly on day at time / custom cron
  - Disk threshold slider (50%-95%, default 80%)
  - Checkboxes for each cleanup target (dangling images, unused images, stopped containers, unused volumes, unused networks)
  - Age threshold for stopped containers (hours, default 24)
  - "Run Cleanup Now" button
- [ ] Cleanup history: last 10 runs with timestamp, resources removed, space reclaimed, status (success/skipped/error)
- [ ] Current disk usage display with a progress bar (updates every 60 seconds)

### API Endpoints

- [ ] `GET /settings/cleanup` — get cleanup configuration
- [ ] `PUT /settings/cleanup` — update cleanup configuration
- [ ] `POST /settings/cleanup/run` — trigger immediate cleanup (admin only)
- [ ] `GET /settings/cleanup/history` — list recent cleanup runs
- [ ] `GET /settings/cleanup/preview` — dry-run: show what would be cleaned up without doing it

### Multi-Server

- [ ] Cleanup runs independently on each server
- [ ] Control plane manages its own container cleanup
- [ ] Worker servers: the agent runs cleanup based on the server's configuration
- [ ] Each server can have different cleanup settings
- [ ] Cleanup history is per-server, viewable on the server detail page
- [ ] For workers: `system.cleanup` agent command triggers cleanup, `system.cleanup_preview` returns dry-run results

### Notifications

- [ ] `system.disk_warning` event when disk exceeds threshold
- [ ] `system.cleanup_completed` event with summary (resources removed, space reclaimed)
- [ ] `system.cleanup_failed` event if cleanup encounters errors

## Technical Notes

- Use bollard's prune endpoints: `image_prune`, `container_prune`, `volume_prune`, `network_prune` with appropriate filters
- The `until` filter on container_prune handles the age threshold for stopped containers
- The `dangling=true` filter on image_prune handles dangling vs. all unused images
- Disk usage: read from `sysinfo` crate (already used for server stats in IF-017) or `statvfs` on the container runtime data directory
- The deploy lock should use the same mechanism as the deploy manager's concurrency control
- Cleanup is idempotent — running it twice in a row is safe (second run finds nothing to clean)

## Out of Scope

- Registry cleanup (cleaning up pushed images from registries)
- Build cache pruning (builder prune) — could be added but is a separate subsystem
- Log file rotation for container logs (handled by the runtime's log driver config)
- Alerting on individual container disk usage (that's monitoring, not cleanup)
- Automatic volume backup before cleanup

## Dependencies

- IF-004 (Container runtime client — prune endpoints via bollard for Docker/Podman)
- IF-043 (Notification system — for disk warning and cleanup events)
- IF-026 (Container metrics — for disk usage data)
