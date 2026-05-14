# IF-222: Cancel in-progress deployment

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

Allow users to cancel a deployment that is currently in progress. If a build is taking too long, stuck, or was triggered accidentally, users should be able to abort without waiting for it to timeout or complete. The running container should remain untouched (cancel = no change to current state).

## Acceptance Criteria

- [ ] "Cancel" button visible on in-progress deployments (deploy detail page and deploy list)
- [ ] Cancellation stops the build process: kills the container image build if in progress
- [ ] If the deploy is past the build phase (container already swapped): cancel is no longer available (too late)
- [ ] Deploy status set to "cancelled"
- [ ] Running container remains unchanged (the old container keeps serving)
- [ ] Clean up: remove any partially built images and temporary containers
- [ ] Cancellation logged in deploy history with "Cancelled by {user}" message
- [ ] API endpoint: `POST /deploys/{id}/cancel`
- [ ] CLI: `icefall deploy cancel` (cancels the most recent in-progress deploy for the app)
- [ ] For multi-server: cancel signal sent to the agent via WebSocket
- [ ] Deploy queue: if deploys are queued, cancel removes from queue without executing

## Technical Notes

- Use a cancellation token / abort signal in the deploy pipeline
- For container image builds: bollard doesn't support cancel directly — kill the build container
- For git clone phase: kill the git process
- Ensure partial state is cleaned up (temporary containers, dangling images)

## Out of Scope

- Automatic timeout-based cancellation (already exists via deploy timeout settings)
- Rollback after cancel (cancel = nothing changed, so no rollback needed)

## Dependencies

- IF-010 (Image build orchestrator)
- IF-011 (Container deployment pipeline)
- IF-131 (Server-aware deploy manager — for multi-server cancel)
