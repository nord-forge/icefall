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
- [ ] Initial set of 20 templates (alphabetical):
  1. Actual Budget
  2. Appsmith
  3. Directus
  4. Ghost
  5. Gitea
  6. Grafana
  7. Jellyfin
  8. Meilisearch
  9. Metabase
  10. MinIO
  11. n8n
  12. Nextcloud
  13. Plausible
  14. Portainer
  15. Supabase (self-hosted)
  16. Umami
  17. Uptime Kuma
  18. Vaultwarden
  19. Vikunja
  20. WikiJS

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
- [ ] Category filter sidebar/chips: All, Analytics, CMS, Database, DevTools, Media, Monitoring, Productivity, Storage
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
