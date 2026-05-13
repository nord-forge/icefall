# IF-187: Config Time Machine — full configuration versioning

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** S

## Description

Version every environment variable, Docker config, and build setting change in SQLite with timestamps. Browse the full history, diff any two points in time, and restore any previous configuration state with one click.

## Acceptance Criteria

- [ ] `config_history` table: `id`, `resource_type` (app/database/settings), `resource_id`, `field`, `old_value` (encrypted), `new_value` (encrypted), `changed_by` (user ID), `changed_at`
- [ ] Record triggered on every update to: env vars, app settings, resource limits, build config, domain changes, database config
- [ ] App settings tab: "History" button → timeline view of all changes
- [ ] Each entry shows: what changed, old → new value, who changed it, when
- [ ] Diff view: select two points in time, see all differences
- [ ] "Restore" button per entry: revert that specific field to its previous value
- [ ] "Restore all" button: revert all fields to their state at a specific timestamp
- [ ] Env var values in history are masked by default (click to reveal, same as current editor)
- [ ] Retention: keep last 100 changes per resource (auto-prune older entries)

## Technical Notes

- This is essentially an audit log for configuration, not just server operations (IF-145 covers server audit)
- Use SQLite triggers or application-level hooks to capture changes
- Encrypted values use the same AES-256-GCM encryption as the current env var storage
- The "restore" action is just another update that gets recorded in history (self-documenting)

## Dependencies

- IF-014 (Env var management)
- IF-002 (Database encryption)
