# IF-204: Migration guides from other platforms

**Phase:** 28 — Comprehensive Docs
**Priority:** Medium
**Estimate:** M

## Description

Write step-by-step migration guides for users coming from other self-hosted PaaS platforms. Each guide maps the source platform's concepts to Icefall equivalents and provides a concrete migration path.

## Guides to Create

### `guides/migrate-from-coolify.mdx`
- [ ] Concept mapping: Coolify projects → Icefall projects, Coolify environments → Icefall environments
- [ ] Export apps: extract Docker configs, env vars, domain settings
- [ ] Import to Icefall: recreate apps, configure env vars, add domains
- [ ] Database migration: dump from Coolify-managed DB, import to Icefall-managed DB
- [ ] DNS cutover: update DNS records from old server to new
- [ ] Verify: health checks, SSL, auto-deploy webhooks

### `guides/migrate-from-dokku.mdx`
- [ ] Concept mapping: Dokku apps → Icefall apps, Dokku plugins → Icefall features
- [ ] Export: `dokku apps:report`, `dokku config:export`, domain/SSL settings
- [ ] Import: create apps, set env vars, configure domains
- [ ] Buildpack → Dockerfile: how Icefall handles builds differently
- [ ] Database migration: pg_dump / mysqldump from Dokku, import to Icefall

### `guides/migrate-from-caprover.mdx`
- [ ] Concept mapping: CapRover apps → Icefall apps, one-click apps → templates
- [ ] Export: app configs, env vars, persistent directories
- [ ] Import: recreate apps, configure volumes, set domains

### `guides/migrate-from-heroku.mdx`
- [ ] Concept mapping: Heroku dynos → Icefall apps, Heroku addons → managed databases
- [ ] Export: `heroku config`, Procfile → build/start commands
- [ ] Heroku-specific adaptations: PORT env var, Procfile parsing, buildpack equivalent
- [ ] Database: `heroku pg:backups:capture` → import to Icefall

### `guides/migrate-from-docker-compose.mdx`
- [ ] Already running Docker Compose on a VPS? Import to Icefall for managed deploys
- [ ] Map Compose services to Icefall apps/databases
- [ ] Preserve volumes and networks
- [ ] Add domains and SSL via Icefall

## Standards

- [ ] Each guide tested end-to-end (deploy on source → migrate → verify on Icefall)
- [ ] Include a "before you start" checklist
- [ ] Estimated migration time
- [ ] Rollback plan: how to go back if something goes wrong
- [ ] Data integrity verification steps

## Dependencies

- IF-047 (Documentation site)
- IF-041 (Server migration — for the Icefall import tools)
