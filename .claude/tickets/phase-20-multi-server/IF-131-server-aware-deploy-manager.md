    # IF-131: Server-aware deploy manager

**Phase:** 20C — Deploy Pipeline
**Priority:** Critical
**Estimate:** L

## Description

Extend the DeployManager to route deploy commands based on the app's `server_id`. When an app is assigned to the control plane, the existing local deploy flow runs unchanged. When an app is assigned to a remote worker, the DeployManager routes all Docker and Caddy commands through the agent WebSocket connection. Build output and deploy status updates from the agent are streamed back through the EventBus so the dashboard receives real-time updates.

## Acceptance Criteria

### Server-Aware Routing
- [ ] DeployManager checks `app.server_id` at the start of every deploy
- [ ] If server_id matches the control-plane server: execute locally (existing flow, no changes)
- [ ] If server_id is a remote worker: dispatch commands via the agent registry

### Remote Deploy Sequence
- [ ] CP sends `build.run` command to agent with: repo URL, branch/commit, env vars, build args
- [ ] Agent clones repo, detects framework, generates Dockerfile, runs `docker build` locally (see IF-132)
- [ ] Agent streams build output back to CP as Event messages
- [ ] `container.create` sent to agent with full ContainerConfig
- [ ] `container.start` sent to agent
- [ ] `health.check` sent to agent (waits for healthy response)
- [ ] `caddy.add_route` sent to agent (or control plane Caddy, depending on routing strategy)
- [ ] Old container stopped and removed on agent (blue-green swap)
- [ ] Each step updates the deploy record status in the database
- [ ] **No image transfer from CP** — builds happen entirely on the worker

### Status Updates
- [ ] Agent sends deploy progress as Event messages
- [ ] Control plane relays these events to the EventBus
- [ ] Dashboard receives deploy status via existing SSE stream
- [ ] Deploy record updated: pending → building → starting → health_check → routing → live → (or failed)
- [ ] No "transferring" status — images are built on the worker, not transferred

### Build Output Streaming
- [ ] If building on worker: agent streams `image.build` output as Events
- [ ] Control plane forwards build output to EventBus for dashboard consumption
- [ ] Build logs stored in the deploy record (same as local builds)

### Error Handling
- [ ] Agent offline: deploy fails immediately with "server unreachable" error
- [ ] Agent response timeout (30s per command): fail the deploy step, update status
- [ ] Health check failure: stop the new container, keep the old one running, mark deploy as failed
- [ ] Any step failure: clean up partially created resources on the agent

### Backward Compatibility
- [ ] Single-server installations (no workers): zero behavior changes
- [ ] Existing deploy API unchanged — server routing is internal

## Technical Notes

- The DeployManager likely needs a `DeployExecutor` trait with `LocalExecutor` and `RemoteExecutor` implementations
- Remote executor uses the agent registry's `send_to(server_id, request)` and awaits the response via the pending request map
- Consider a `DeployContext` struct that carries server_id, agent connection, and deploy state through the pipeline
- No "transferring" status needed — the agent builds locally, so the flow goes directly from "building" to "starting"

## Out of Scope

- Multi-server deploys (deploying the same app to multiple servers simultaneously)
- Canary or percentage-based rollouts across servers
- Automatic server selection based on resource availability (server is explicitly set on the app)
- Rollback across servers (use re-deploy for now)

## Dependencies

- IF-119 (agent WebSocket endpoint and registry for dispatching commands)
- IF-125 (agent Docker operations handler for receiving and executing commands)
