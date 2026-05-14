# IF-185: Drift Detective — continuous config reconciliation

**Phase:** 26 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

The agent continuously checksums running container configs, env vars, and network rules against the declared state in SQLite. Any out-of-band change (manual container exec, someone SSH'd in and edited something) triggers an alert with a diff and one-click revert to the declared state.

## Acceptance Criteria

- [ ] Agent reconciliation loop: every 60 seconds, compare running container state vs declared state
- [ ] Checked properties per container: env vars, resource limits, network connections, port mappings, labels, image tag
- [ ] Drift detected: dispatch `system.drift_detected` notification with a structured diff
- [ ] Dashboard: "Drift" badge on affected app cards
- [ ] Drift detail view: side-by-side diff (declared vs actual) for each drifted property
- [ ] "Revert to declared state" button: recreate the container with the declared config
- [ ] "Accept current state" button: update the declared state to match the running container
- [ ] Drift history: log of all detected drifts with timestamps
- [ ] Per-app setting: "Monitor for drift" toggle (default: on)

## Technical Notes

- Use container inspect (Docker/Podman via bollard) to get the running container's config, compare against the app record in SQLite
- Env var comparison: the declared env vars are encrypted in the DB — decrypt, compare, report
- For multi-server: each agent runs its own reconciliation loop and reports drifts to the control plane

## Dependencies

- IF-004 (Container runtime client — container inspect via Docker/Podman)
- IF-043 (Notification system)
- IF-127 (Agent metrics — piggyback on the agent's existing polling loop)
