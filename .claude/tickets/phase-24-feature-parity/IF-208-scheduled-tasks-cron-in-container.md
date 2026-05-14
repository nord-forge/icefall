# IF-208: Scheduled tasks (cron-in-container)

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** M

## Description

Allow users to define cron-like scheduled tasks that execute inside a running container. Common use cases: database vacuum, cache purge, report generation, sitemap rebuild. Users currently have to bake cron into Dockerfiles which is fragile and invisible to the platform.

## Acceptance Criteria

- [ ] `scheduled_tasks` table: `id`, `app_id`, `name`, `command`, `cron_expression`, `timeout_seconds` (default 300), `enabled`, `container_name` (optional, for Compose apps), `created_at`, `updated_at`
- [ ] `scheduled_task_executions` table: `id`, `task_id`, `status` (running/success/failed/timed_out), `output` (text), `started_at`, `finished_at`
- [ ] Background scheduler evaluates cron expressions and dispatches tasks
- [ ] Tasks execute via `container exec (Docker/Podman)` in the target container (reuse IF-077/IF-163 exec mechanism)
- [ ] For multi-server: task executes on the server where the container runs (via agent)
- [ ] App detail page: new "Scheduled Tasks" tab listing tasks with last-run status
- [ ] Task CRUD form: name, command, cron expression (with human-readable preview), timeout, enabled toggle
- [ ] "Run Now" button for manual execution
- [ ] Execution history list with output viewer (expandable rows)
- [ ] API endpoints: `GET/POST /apps/{id}/scheduled-tasks`, `PUT/DELETE /apps/{id}/scheduled-tasks/{task_id}`, `POST .../run`, `GET .../executions`
- [ ] Wire task success/failure to notification dispatch (IF-043)

## Technical Notes

- Use a cron parser crate (e.g., `cron` or `croner`) for expression evaluation
- The background scheduler can be a tokio task that checks every 60 seconds
- Capture stdout+stderr from exec, truncate at 1MB to prevent DB bloat
- Execution history retention: keep last 100 executions per task, prune older

## Out of Scope

- Host-level cron (tasks run inside containers only)
- Task dependencies / DAGs
- Parallel execution of the same task

## Dependencies

- IF-077 (Container terminal — container exec mechanism)
- IF-163 (Post-deploy commands — same exec infrastructure)
