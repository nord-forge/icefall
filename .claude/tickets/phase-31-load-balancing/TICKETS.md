# Phase 31: Load Balancing

> **Status: SHIPPED** — all tickets complete and merged to `main` (2026-05-18).
> Priority: v2.0
> Estimated effort: L (1-2 weeks)
> Dependencies: Phase 20 (multi-server) — already shipped

## Overview

Add load balancing so a single app can run on multiple servers with traffic distributed across instances. Currently, each app deploys to exactly one server. This phase adds multi-instance deployment, Caddy multi-upstream routing, health-aware traffic distribution, and a scaling UI.

## Current State

- **Multi-server is shipped** (Phase 20): server registration, remote Docker via agent, app-to-server assignment, server migration
- **Caddy routing** supports single upstream per domain: `add_route(domain, upstream)` and `update_route(domain, upstream)` in `src/caddy/routes.rs`
- **No multi-upstream support**: Caddy's `reverse_proxy` directive natively supports multiple upstreams with health checks, but the current code only passes one
- **App model** has `server_id: Option<String>` — singular, not a list
- **Deploy manager** deploys to one server per app via `DeployExecution`
- **Server forecast** exists (`src/api/routes/forecast.rs`) — can inform placement decisions

## Tickets

| ID | Title | Priority | Size | Dependencies | Status |
|---|---|---|---|---|---|
| [BE-257](BE-257-multi-instance-app-model.md) | Multi-instance app model | Critical | M | None | Shipped |
| [BE-258](BE-258-multi-instance-deploy-pipeline.md) | Multi-instance deploy pipeline | Critical | L | BE-257 | Shipped |
| [BE-259](BE-259-caddy-multi-upstream-routing.md) | Caddy multi-upstream routing | Critical | M | BE-258 | Shipped |
| [BE-260](BE-260-instance-health-monitoring.md) | Instance health monitoring | High | M | BE-257 | Shipped |
| [BE-261](BE-261-scaling-api.md) | Scaling API | High | S | BE-258 | Shipped |
| [FE-262](FE-262-scaling-instances-ui.md) | Scaling and instances UI | High | L | BE-261 | Shipped |
| [FE-263](FE-263-server-capacity-visualization.md) | Server capacity visualization | Medium | S | BE-261 | Shipped |
| [QA-264](QA-264-load-balancing-tests.md) | Load balancing integration tests | High | M | BE-258, BE-259 | Shipped |

## Dependency Graph

```
BE-257 (app model)
  ├── BE-258 (deploy pipeline)
  │     ├── BE-259 (Caddy multi-upstream)
  │     ├── BE-261 (scaling API)
  │     │     ├── FE-262 (scaling UI)
  │     │     └── FE-263 (server capacity)
  │     └── QA-264 (integration tests)
  └── BE-260 (instance health)
```
