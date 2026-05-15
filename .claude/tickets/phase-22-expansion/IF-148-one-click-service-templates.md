# IF-148: One-click service templates

**Phase:** 22 — Expansion (v1.2)
**Priority:** High
**Estimate:** L

## Description

Build a template system that lets users deploy popular self-hosted services in one click. Each template is a Compose file with metadata, sensible defaults, and required configuration prompts. Templates ship as a bundled JSON catalog with the Icefall binary and can be refreshed from a remote repository. The deployment mechanism is the existing Docker Compose support (IF-073).

## Acceptance Criteria

### Template Format

- [ ] Each template is a directory containing:
  - `template.json` — metadata (name, description, version, icon URL, categories, required_inputs, default_env, min_resources)
  - `docker-compose.yml` — the Compose file with `${VARIABLE}` placeholders for user inputs
  - `README.md` — optional setup notes shown post-deploy
- [ ] `template.json` schema:
  ```json
  {
    "id": "plausible",
    "name": "Plausible Analytics",
    "description": "Privacy-friendly Google Analytics alternative",
    "version": "2.1.4",
    "icon": "https://...",
    "categories": ["analytics"],
    "website": "https://plausible.io",
    "required_inputs": [
      { "key": "BASE_URL", "label": "Your domain", "type": "url", "placeholder": "https://analytics.example.com" },
      { "key": "ADMIN_EMAIL", "label": "Admin email", "type": "email" }
    ],
    "default_env": {
      "DISABLE_REGISTRATION": "invite_only"
    },
    "min_resources": { "memory_mb": 512, "disk_mb": 1024 },
    "volumes": ["db-data", "event-data"],
    "ports": [8000]
  }
  ```
- [ ] Compose file uses `${VARIABLE}` syntax — Icefall substitutes required_inputs + default_env before deploying

### Template Catalog

- [ ] Bundled catalog: templates compiled into the binary as embedded assets (or shipped alongside in a `templates/` directory)
- [ ] Remote catalog refresh: `GET /api/v1/templates/refresh` fetches the latest catalog from a configurable URL (default: Icefall's GitHub repo releases)
- [ ] Catalog stores: template metadata + last_updated timestamp in SQLite
- [x] Initial set of 50 templates seeded via migration `20260520000007_seed_service_templates.sql`:

  **DevTools (13)**
  1. Gitea — lightweight Git hosting
  2. Portainer — Docker management UI
  3. Woodpecker CI — CI/CD with YAML pipelines
  4. Traefik — reverse proxy with auto-SSL
  5. Docker Registry — private container registry
  6. Mailpit — email testing SMTP + web UI
  7. Unleash — feature flag management
  8. Hoppscotch — API development platform
  9. PrivateBin — zero-knowledge encrypted pastebin
  10. Verdaccio — private npm registry
  11. Appsmith — low-code internal tools
  12. Forgejo — community-governed Git forge
  13. Gitness — Git + CI in one platform

  **Productivity (8)**
  14. n8n — workflow automation
  15. Wiki.js — team wiki with markdown
  16. Outline — modern team knowledge base
  17. Shlink — URL shortener with analytics
  18. Cal.com — scheduling (Calendly alternative)
  19. Vikunja — kanban / task management
  20. Rallly — meeting scheduling (Doodle alternative)
  21. Actual Budget — personal finance tracking

  **Monitoring (6)**
  22. Uptime Kuma — uptime monitoring + status pages
  23. Grafana — observability dashboards
  24. Prometheus — metrics collection + alerting
  25. GlitchTip — Sentry-compatible error tracking
  26. Grafana Loki — log aggregation
  27. Gatus — health dashboard + status page

  **Database (5)**
  28. Meilisearch — typo-tolerant search engine
  29. Supabase — open-source Firebase alternative
  30. pgAdmin — PostgreSQL admin GUI
  31. Adminer — universal lightweight DB admin
  32. DbGate — modern multi-database GUI

  **Storage (4)**
  33. MinIO — S3-compatible object storage
  34. Nextcloud — file sync + collaboration
  35. PicoShare — minimal file sharing
  36. Duplicati — encrypted cloud backups

  **Security (4)**
  37. Vaultwarden — Bitwarden-compatible password manager
  38. Infisical — secrets management platform
  39. Authentik — SSO / identity provider
  40. CrowdSec — collaborative intrusion prevention

  **Analytics (3)**
  41. Plausible — privacy-friendly web analytics
  42. Metabase — BI with visual query builder
  43. Umami — lightweight privacy analytics

  **Communication (2)**
  44. Ntfy — push notifications via HTTP
  45. Listmonk — newsletter + mailing list manager

  **CMS (2)**
  46. Directus — headless CMS with REST + GraphQL
  47. Ghost — blog + newsletter + membership

  **AI/ML (2)**
  48. Ollama — run LLMs locally
  49. Open WebUI — ChatGPT-like UI for Ollama

  **Media (1)**
  50. Immich — photo/video backup with ML

### API Endpoints

- [ ] `GET /api/v1/templates` — list all templates with metadata (paginated, filterable by category)
- [ ] `GET /api/v1/templates/{id}` — get template detail including required_inputs and README
- [ ] `POST /api/v1/templates/{id}/deploy` — deploy a template with user-provided inputs
  - Request body: `{ "name": "my-analytics", "server_id": "...", "project_id": "...", "inputs": { "BASE_URL": "...", "ADMIN_EMAIL": "..." } }`
  - Validates all required_inputs are provided
  - Checks server has sufficient resources (if min_resources specified)
  - Creates a Compose-type app, substitutes variables, triggers deploy
  - Returns the created app with deploy status
- [ ] `POST /api/v1/templates/refresh` — refresh catalog from remote (admin only)

### Dashboard UI

- [ ] Template browser: grid of cards with icon, name, description, category badge
- [ ] Category filter sidebar/chips: All, AI/ML, Analytics, CMS, Communication, Database, DevTools, Media, Monitoring, Productivity, Security, Storage
- [ ] Search by name
- [ ] Template detail drawer/modal: description, README content, resource requirements, version info
- [ ] Deploy form: auto-generated from `required_inputs` — each input renders as the appropriate form field (text, email, url, password, number)
- [ ] Server selection (if multi-server) and project assignment
- [ ] Deploy button triggers creation and redirects to the new app's deploy view
- [ ] "One-Click Services" entry in sidebar navigation
- [ ] Template card shows "Deployed" badge if the user already has an instance running

### Template Updates

- [ ] When a template version changes, show an "Update available" badge on the deployed app's overview
- [ ] Update action: re-pulls images with the new tag, preserves volumes and env vars
- [ ] No automatic updates — user must explicitly trigger

## Technical Notes

- Templates are essentially pre-configured Compose deployments — reuse the entire IF-073 Compose pipeline
- Store the template ID + version on the app record so we can track which apps came from templates
- For the bundled catalog, consider `include_dir!` macro or a build step that embeds the templates directory
- Template icons: store locally or proxy through Icefall to avoid mixed-content issues
- The remote catalog URL should default to a GitHub raw URL pointing to a `templates/catalog.json` in the Icefall repo

## Out of Scope

- Community template submissions (UI for submitting templates)
- Template versioning with migration scripts between versions
- Multi-template stacks (deploying multiple related templates together)
- Template marketplace with ratings/reviews
- Custom user-created templates (users can just use Compose directly)

## Dependencies

- IF-073 (Docker Compose support)
- IF-074 (Projects — for project assignment during deploy)
- IF-135 (Server selection — for multi-server template deploys)
