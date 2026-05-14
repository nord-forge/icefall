# IF-180: App dependency graph

**Phase:** 26 — Icefall+
**Priority:** Low
**Estimate:** M

## Description

Visualize the relationships between apps and databases as an interactive dependency graph. Shows which apps are linked to which databases, which services share environment variables, and which containers are on the same container network. Helps users understand their infrastructure at a glance.

## Acceptance Criteria

- [ ] New "Infrastructure" page accessible from the sidebar
- [ ] Interactive node-link diagram:
  - App nodes (colored by status: running/stopped/deploying)
  - Database nodes (colored by type: postgres/mysql/redis/etc.)
  - Domain nodes (linked to their apps)
  - Edges: app→database links, Compose service relationships, shared networks
- [ ] Click a node: navigate to the app/database detail page
- [ ] Hover a node: show name, status, server, resource usage
- [ ] Filter by project, server, or status
- [ ] Layout: force-directed graph or hierarchical layout
- [ ] Server grouping: containers on the same server visually grouped
- [ ] Zoom/pan for large deployments
- [ ] Export as SVG (for documentation)

## Technical Notes

- Use D3.js force simulation or a lightweight graph library (cytoscape.js, vis-network)
- Data sourced from existing APIs: apps list, database links, Compose relationships, domain assignments
- For small deployments (1-5 apps): simple layout. For larger: force-directed with server clustering.
- Preact island with `client:load` — the graph is inherently interactive

## Dependencies

- IF-029 (Database linking — app→db relationships)
- IF-073 (Compose support — service relationships)
- IF-074 (Projects — grouping context)
