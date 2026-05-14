# IF-214: Container cleanup configuration per server

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

IF-152 covers automated container cleanup as a global feature. This ticket extends it with per-server configuration: custom cleanup frequency, disk threshold, and granular control over what gets cleaned (images, volumes, networks). Also adds cleanup execution history and a manual cleanup button per server. Works with both Docker and Podman runtimes.

## Acceptance Criteria

- [ ] Per-server fields on `servers` table: `container_cleanup_frequency` (cron, default `0 */6 * * *`), `container_cleanup_threshold` (disk %, default 80), `container_cleanup_enabled` (bool)
- [ ] Granular toggles: `cleanup_unused_images`, `cleanup_unused_volumes`, `cleanup_unused_networks`, `cleanup_dangling_only` (default true — only dangling images vs all unused)
- [ ] `force_container_cleanup` toggle: ignore threshold, always run on schedule
- [ ] Server settings page: Container cleanup section with all fields above
- [ ] "Run Cleanup Now" button: triggers immediate cleanup on that server
- [ ] `container_cleanup_executions` table: `id`, `server_id`, `started_at`, `finished_at`, `space_reclaimed_bytes`, `images_removed`, `volumes_removed`, `networks_removed`, `status`
- [ ] Execution history list in server settings (last 20 runs)
- [ ] Deploy-aware: skip cleanup if a deploy is in progress on that server
- [ ] For workers: cleanup commands sent via agent WebSocket
- [ ] Wire cleanup success/failure to notification dispatch

## Technical Notes

- Uses system prune / image prune / volume prune / network prune via the container runtime abstraction (bollard for Docker, podman API for Podman)
- Check disk usage before running (skip if below threshold unless forced)
- Application image retention: keep the last N images per app (configurable, default 5) — don't prune actively used rollback images

## Out of Scope

- Build cache cleanup (builder prune)
- Container log rotation (handled by container runtime daemon config)

## Dependencies

- IF-152 (Automated container cleanup — base implementation)
- IF-127 (Agent metrics — disk usage data)
