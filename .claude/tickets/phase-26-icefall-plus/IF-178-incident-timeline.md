# IF-178: Incident timeline & status page

**Phase:** 26 — Icefall+
**Priority:** Medium
**Estimate:** L

## Description

Build an integrated incident timeline that correlates deploy events, health check failures, server status changes, and error spikes into a single chronological view. Optionally expose a public status page for each app. No other PaaS includes built-in incident tracking — users typically need Statuspage.io or Betteruptime separately.

## Acceptance Criteria

### Incident Timeline (Internal)
- [ ] New "Incidents" page accessible from the sidebar
- [ ] Auto-detected incidents based on:
  - Health check failure (3+ consecutive failures)
  - Deploy failure
  - Server going offline
  - Container restart (OOM kill or crash loop)
- [ ] Each incident shows: start time, duration, affected apps/servers, root cause (if detectable), resolution
- [ ] Timeline visualization: horizontal bar chart showing incident duration overlaid with deploy markers
- [ ] Incident detail: correlated events (health checks, deploys, logs) in chronological order
- [ ] Manual incident creation for events the system can't auto-detect
- [ ] Incident status: `investigating`, `identified`, `monitoring`, `resolved`
- [ ] Incident notes: timestamped updates (like a mini postmortem log)

### Public Status Page (Optional)
- [ ] Per-app opt-in: "Enable public status page" toggle in app settings
- [ ] Public URL: `{base-domain}/status/{app-slug}` (no auth required)
- [ ] Shows: current status (operational/degraded/down), uptime percentage (30 days), incident history
- [ ] Reuses the uptime timeline component (IF-028) in a minimal public layout
- [ ] No Icefall branding on the status page (or minimal "Powered by Icefall" footer)
- [ ] Custom status page domain support (CNAME to the base domain)

### API Endpoints
- [ ] `GET /incidents` — list incidents (paginated, filterable by status/app/server)
- [ ] `POST /incidents` — create manual incident
- [ ] `PUT /incidents/{id}` — update incident status/notes
- [ ] `GET /incidents/{id}/events` — get correlated events for an incident
- [ ] `GET /status/{app-slug}` — public status data (JSON) for the status page

## Technical Notes

- Incident auto-detection runs as a background task, checking health events and deploy status every 30 seconds
- Correlation logic: if a health check fails within 5 minutes of a deploy, link them as the same incident
- The public status page is a separate Astro page with no auth middleware — just reads from the incidents/health tables
- Incident storage: new `incidents` and `incident_events` tables

## Dependencies

- IF-025 (Health check system — event data)
- IF-028 (Uptime timeline UI — reuse component)
- IF-144 (Offline server handling — server status data)
