# IF-211: Pre-deployment commands

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

Allow users to configure commands that run before the container swap during deployment. Runs in a temporary container from the new image. Common use cases: database migrations, pre-flight checks, schema validation. Complements IF-163 (post-deploy commands) which runs after the swap.

## Acceptance Criteria

- [ ] `pre_deploy_commands` text field on the `apps` table (nullable, newline-separated)
- [ ] `pre_deploy_container` field: which container to run in (for Compose apps, defaults to the primary service)
- [ ] App settings tab: textarea for pre-deploy commands alongside the post-deploy commands from IF-163
- [ ] During deploy: after new image is built/pulled but BEFORE the blue-green swap, spin up a temporary container from the new image and run each command sequentially
- [ ] Command output captured and shown as a "Pre-deploy" step in deploy logs (SSE streamed)
- [ ] If a pre-deploy command exits non-zero: deploy **fails** and the swap does NOT happen (old container stays running)
- [ ] Failed pre-deploy triggers deploy failure notification
- [ ] Timeout: 5 minutes per command (configurable)
- [ ] For multi-server: pre-deploy runs on the target server via agent

## Technical Notes

- Spin up the temporary container with the same env vars and volumes as the target
- The temporary container is removed after pre-deploy commands complete (success or failure)
- This is architecturally different from post-deploy: pre-deploy failure is a hard stop

## Out of Scope

- Conditional commands (first deploy vs subsequent)
- Interactive commands
- Running pre-deploy in the OLD container

## Dependencies

- IF-011 (Container deployment pipeline)
- IF-163 (Post-deploy commands — same settings UI section)
