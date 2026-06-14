# BE-258: Multi-instance deploy pipeline

**Phase:** 31
**Priority:** Critical
**Size:** L
**Dependencies:** BE-257

## Description

Modify the deploy pipeline to build once, then distribute the image to N servers and start containers on each.

## Changes to `src/deploy/manager/`

- After build: push image to a shared registry or transfer to target servers
- For each target server: pull image, start container, record host_port
- Collect all `(server_host, host_port)` pairs as upstream list
- Pass upstream list to Caddy route update
- Rolling deploy: update instances one at a time (configurable: all-at-once or rolling)
- Instance health check before marking deploy as running

## Image Distribution Strategy

- Build on control plane (or designated build server)
- Save image as tarball, transfer via agent API to each target server
- Agent loads tarball into local Docker
- This avoids requiring a private registry for small setups

## Acceptance Criteria

- Given an app with desired_instances = 2 and 2 available servers, when deployed, then containers run on both servers
- Given a rolling deploy, when instance 1 is updated, then instance 2 continues serving traffic until instance 1 is healthy
- Given a build, when the image is transferred to a remote server, then it loads and starts correctly

## Out of Scope

Private registry integration (future optimization), cross-region deployment
