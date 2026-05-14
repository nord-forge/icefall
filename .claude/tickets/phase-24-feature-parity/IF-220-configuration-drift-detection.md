# IF-220: Configuration drift detection (redeploy prompt)

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

When a user changes app configuration (env vars, resource limits, domains, health checks) in the dashboard, the running container still uses the old config until redeployed. Users often change settings and assume they take effect immediately. This ticket adds a visible banner that detects when the saved configuration differs from the running container's configuration, prompting the user to redeploy.

## Acceptance Criteria

- [ ] After any config change that requires a redeploy: show a persistent banner on the app detail page — "Configuration has changed. Redeploy to apply."
- [ ] Banner includes a "Redeploy Now" button
- [ ] Track `config_hash` on the app model: SHA-256 of serialized deploy-relevant fields (env vars, resource limits, build config, volumes, domains)
- [ ] On each deploy completion: store the deployed config hash in the `deploys` table
- [ ] Compare current config hash vs last-deployed config hash to determine drift
- [ ] Banner auto-dismisses after a successful deploy
- [ ] Changes that DON'T require redeploy (app name, description, tags) don't trigger the banner
- [ ] API endpoint: `GET /apps/{id}/drift` returns `{ drifted: bool, fields: ["env_vars", "resource_limits"] }`
- [ ] Optional: show which specific fields changed since last deploy

## Technical Notes

- Config hash should cover: env vars, resource limits, build command, install command, start command, Dockerfile, volumes, port mappings
- Don't include: name, description, tags, webhook settings (these don't affect the running container)
- Store the hash as a column on the `apps` table (`deployed_config_hash`) updated on each successful deploy

## Out of Scope

- Detecting changes made INSIDE the container (file edits via exec) — that's IF-185 Drift Detective
- Auto-redeploy on config change
- Config change history / audit log (that's IF-187 Config Time Machine)

## Dependencies

- IF-014 (Environment variable management)
- IF-011 (Container deployment)
