# IF-192: Portable App Bundles (.icefall files)

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Export a fully self-contained app definition as a shareable `.icefall` file. Anyone with Icefall can import it and have the app configured in seconds. Includes Docker config, env var templates, build settings, resource limits, health checks, and domain patterns — everything except secrets.

## Acceptance Criteria

- [ ] Export: "Export as bundle" button on app settings → downloads `{app-name}.icefall` file
- [ ] Bundle format: JSON with a defined schema version:
  ```json
  {
    "icefall_bundle": "1.0",
    "app": {
      "name": "my-api",
      "type": "git",
      "repo_url": "https://github.com/user/repo",
      "branch": "main",
      "framework": "next",
      "build_command": "npm run build",
      "start_command": "npm start",
      "port": 3000,
      "base_directory": null
    },
    "env_template": [
      { "key": "DATABASE_URL", "description": "PostgreSQL connection string", "required": true },
      { "key": "REDIS_URL", "description": "Redis connection string", "required": false, "default": "redis://localhost:6379" }
    ],
    "resources": { "memory_mb": 512, "cpu_shares": 256 },
    "health_check": { "path": "/health", "interval_secs": 30 },
    "volumes": [{ "container_path": "/data", "description": "Persistent data" }],
    "post_deploy": ["npx prisma migrate deploy"]
  }
  ```
- [ ] Import: "Import from bundle" option in app creation → upload `.icefall` file
- [ ] Import flow: pre-fill the creation wizard, prompt for required env var values, deploy
- [ ] Env template: required vars show as mandatory fields, optional vars show with defaults
- [ ] Secrets are NEVER included in bundles — only env var names, descriptions, and non-secret defaults
- [ ] CLI: `icefall export <app-name>` and `icefall import <bundle-file>`
- [ ] Share URL: optional "Create share link" that uploads the bundle to the Icefall instance's public URL

## Technical Notes

- The `.icefall` file is just JSON — keep it human-readable and editable
- Schema versioning ensures forward compatibility
- For Docker image apps: include `image`, `tag`, `port`, `env_template`
- For Compose apps: include the full `docker-compose.yml` content inline

## Dependencies

- IF-018 (App creation flow — import fills the wizard)
- IF-038 (CLI — export/import commands)
