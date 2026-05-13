# IF-205: Research — container runtime alternatives (Podman, containerd, Docker)

**Phase:** 23 — Rust Quality & Performance
**Priority:** Critical
**Estimate:** M

## Description

Investigate whether Icefall should stick with Docker Engine or switch to an alternative container runtime (Podman, containerd direct, or a hybrid approach). This decision affects the entire codebase — the Docker client (IF-004), deploy pipeline, agent, install script, and all container operations. Must be resolved before starting any other Phase 22-27 work.

## Why This Matters

- Docker Engine is a daemon with overhead (~50-100MB RAM idle, root-required by default)
- Podman is daemonless, rootless-capable, and CLI-compatible with Docker
- containerd is the low-level runtime Docker itself uses — cutting out the Docker daemon removes a layer
- OrbStack (macOS only) proves that alternative runtimes can be significantly faster
- For Icefall's target audience ($5-$20 VPS), every MB of RAM and every ms of startup time matters
- If we switch, it affects: IF-004 (Docker client), IF-073 (Compose), IF-077 (terminal), IF-152 (cleanup), IF-183 (Ghost Mode), and every deploy-related ticket

## Research Questions

### 1. Runtime Comparison

| Criteria | Docker Engine | Podman | containerd (direct) |
|----------|--------------|--------|-------------------|
| Idle RAM overhead | ? | ? | ? |
| Container start latency | ? | ? | ? |
| Image build support | ? | ? | ? |
| Compose support | ? | ? | ? |
| Rootless mode | ? | ? | ? |
| Availability on common VPS distros | ? | ? | ? |
| Rust client library maturity | bollard (mature) | ? | ? |
| WebSocket exec (terminal) | ? | ? | ? |
| Health check support | ? | ? | ? |
| Volume management | ? | ? | ? |

### 2. Podman Specific
- [ ] Can Podman run as a drop-in Docker replacement on Ubuntu/Debian/CentOS?
- [ ] Does `podman-compose` or `podman compose` cover our Docker Compose needs (IF-073)?
- [ ] Is there a mature Rust client for Podman? (Podman's API is Docker-compatible — does bollard work?)
- [ ] Rootless containers: security benefits, but can Caddy still route to them?
- [ ] Podman's systemd integration: could each container be a systemd unit (better lifecycle management)?
- [ ] Pod concept: does grouping containers into pods benefit Compose stacks?

### 3. containerd Direct
- [ ] Would using containerd directly (via `containerd-client` Rust crate) reduce overhead?
- [ ] What do we lose vs Docker? (Compose, image building, CLI familiarity)
- [ ] Is `nerdctl` (containerd's Docker-compatible CLI) mature enough?
- [ ] Image building without Docker: BuildKit standalone, or keep Docker for builds only?

### 4. Hybrid Approach
- [ ] Could Icefall support multiple runtimes with an abstraction layer?
- [ ] E.g., Docker for development/familiarity, Podman for production/security
- [ ] How much abstraction overhead does this add? Is it worth the complexity?
- [ ] Could the install script auto-detect which runtime is available?

### 5. Migration Impact
- [ ] If we switch from Docker: what changes in the install script (IF-040)?
- [ ] Does the agent binary (IF-121) need separate runtime support?
- [ ] How do existing users migrate? Can we convert Docker containers to Podman without data loss?
- [ ] Volume compatibility: do Docker volumes work with Podman and vice versa?

## Deliverables

- [ ] Benchmark results: idle RAM, container start latency, image build time for each runtime
- [ ] Compatibility matrix: which Icefall features work with each runtime
- [ ] Rust client library assessment: bollard vs alternatives, API coverage gaps
- [ ] Recommendation: stick with Docker, switch to Podman, switch to containerd, or support multiple
- [ ] If recommending a switch: migration plan and list of affected tickets
- [ ] Research document in `.claude/research/container-runtime-evaluation.md`

## Benchmarking Methodology

- [ ] Test server: fresh Ubuntu 24.04 on a Hetzner CAX11 (ARM64, 2 vCPU, 4GB RAM)
- [ ] Baseline: measure idle system RAM, then install each runtime and measure again
- [ ] Container start: time from `create` to `running` for a simple Node.js app (10 runs, median)
- [ ] Image build: build the same Dockerfile (multi-stage Node.js) on each runtime (3 runs, median)
- [ ] Compose stack: deploy a 3-service Compose stack on each runtime, measure total time
- [ ] Concurrent containers: start 20 containers simultaneously, measure total time and peak RAM

## Out of Scope

- Kubernetes/K3s evaluation (fundamentally different architecture)
- Firecracker/microVMs (interesting but too niche for v1.x)
- Windows container support

## Dependencies

- None — this is a prerequisite for all other work

## Blocking

This ticket BLOCKS the following work:
- IF-152 (Docker cleanup) — cleanup commands differ per runtime
- IF-183 (Ghost Mode) — container suspend/wake mechanism may differ
- IF-173 (Raw Compose mode) — Compose support differs between runtimes
- IF-165 (Database terminal) — exec mechanism may differ
- IF-172 (TCP proxy) — networking model may differ
- All Phase 22-27 tickets that touch containers
