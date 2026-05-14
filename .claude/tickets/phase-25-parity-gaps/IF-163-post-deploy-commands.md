# IF-163: Post-deployment commands

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** M

## Description

Allow users to configure commands that run inside the container after a successful deployment. Common use cases: database migrations, cache warming, asset compilation, seed data. Commands execute via container exec (Docker/Podman) in the newly deployed container.

## Acceptance Criteria

- [ ] `post_deploy_commands` text field on the `apps` table (nullable, newline-separated commands)
- [ ] App settings tab: textarea for post-deploy commands with placeholder example
- [ ] After container starts and health check passes, execute each command sequentially via container exec (Docker/Podman)
- [ ] Command output captured and appended to the deploy log (SSE streamed)
- [ ] If a command exits non-zero: deploy marked as "deployed with warnings" (not failed — container is already running)
- [ ] Deploy detail shows post-deploy step with pass/fail per command
- [ ] Commands run as the container's default user (not root, unless the container runs as root)
- [ ] Timeout: 5 minutes per command (configurable in settings)
- [ ] For multi-server: commands execute on the server where the container is running (via agent)

## Technical Notes

- Use bollard's `exec_create` + `exec_start` APIs (already used by IF-077 container terminal)
- Stream command output via the existing deploy SSE pipeline
- Post-deploy commands should NOT run on rollback deploys (the rolled-back container is already known-good)

## Out of Scope

- Pre-deploy commands (run before container swap)
- Conditional commands (run only on first deploy vs. subsequent)
- Interactive commands (no TTY allocation)

## Dependencies

- IF-011 (Container deployment)
- IF-077 (Container terminal — uses the same container exec mechanism)
