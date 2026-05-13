# IF-188: Deploy Replay — structured deploy event streams

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** S

## Description

Record every deploy as a structured event stream: git diff summary, build log, health check results, resource usage delta, timing breakdown. "Replay" any past deploy to see exactly what happened, or diff two deploys to understand what changed between them.

## Acceptance Criteria

- [ ] `deploy_events` table: `id`, `deploy_id`, `event_type`, `data` (JSON), `timestamp`
- [ ] Event types captured per deploy:
  - `git.diff_summary` — files changed, insertions, deletions
  - `build.step` — each build step with timing
  - `build.image_size` — final image size in bytes
  - `container.resource_delta` — memory/CPU change vs previous deploy
  - `health.result` — health check pass/fail with timing
  - `canary.result` — canary probe results (if IF-186 enabled)
  - `deploy.timing` — total time, build time, swap time, health check time
- [ ] Deploy detail page: "Replay" tab showing the event stream as a timeline
- [ ] Deploy diff: select two deploys → side-by-side comparison of all metrics
- [ ] "What changed?" summary: auto-generated one-liner for each deploy (e.g., "3 files changed, build 12s faster, +5MB image size")
- [ ] API: `GET /deploys/{id}/events` returns structured event stream

## Technical Notes

- Events are emitted during the deploy pipeline — each step already logs to SSE, this captures them to SQLite as structured data instead of just streamed text
- Git diff summary: `git diff --stat HEAD~1` during the build step
- Image size: `docker inspect` after build
- Resource delta: compare memory/CPU of new container vs previous deploy's container

## Dependencies

- IF-011 (Container deployment)
- IF-022 (Deploy view)
