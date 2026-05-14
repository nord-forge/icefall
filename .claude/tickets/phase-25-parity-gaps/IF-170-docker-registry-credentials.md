# IF-170: Container registry credentials & management

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** M

## Description

Allow users to configure credentials for private container registries so they can deploy private images (IF-065) and optionally push built images to a registry. Supports Docker Hub, GitHub Container Registry (GHCR), GitLab Registry, and custom OCI-compliant registries. Works with both Docker and Podman runtimes.

## Acceptance Criteria

### Database
- [ ] New `registries` table: `id`, `name`, `url`, `username` (encrypted), `password` (encrypted), `type` (enum: dockerhub, ghcr, gitlab, custom), `created_at`, `updated_at`

### Registry Operations
- [ ] CRUD for registry credentials
- [ ] Test connection: verify credentials by calling the registry's `/v2/` endpoint
- [ ] Presets for common registries:
  - Docker Hub: `https://index.docker.io/v1/`
  - GHCR: `https://ghcr.io`
  - GitLab: `https://registry.gitlab.com`
  - Custom: user-provided URL

### Pull Integration
- [ ] When deploying a container image app (IF-065): if the image URL matches a configured registry, use those credentials for image pull
- [ ] Registry selection dropdown on the container image deploy form
- [ ] Credentials passed to bollard's `create_image` with auth config

### Push Integration (optional)
- [ ] App settings: "Push to registry" toggle with registry selector and tag pattern
- [ ] After successful build: tag image with registry URL + configured tag pattern (e.g., `ghcr.io/user/app:latest`, `ghcr.io/user/app:{commit-sha}`)
- [ ] Push image to configured registry
- [ ] Push status shown in deploy log

### Dashboard UI
- [ ] Settings page: "Container Registries" section
- [ ] Registry list: name, URL, type icon, test status
- [ ] Add registry form: type selector (with preset URL fill), username, password, name
- [ ] "Test Connection" button
- [ ] App settings: registry dropdown for image deploys

### API Endpoints
- [ ] `GET /registries` — list registries (credentials masked)
- [ ] `POST /registries` — create registry
- [ ] `PUT /registries/{id}` — update registry
- [ ] `DELETE /registries/{id}` — delete registry
- [ ] `POST /registries/{id}/test` — test connection

## Technical Notes

- Bollard's `create_image` accepts `AuthConfig` with username/password for authenticated pulls
- For push: bollard's `push_image` with the same auth config
- Credentials encrypted at rest using existing AES-256-GCM
- For multi-server: registry credentials synced to agents (encrypted via envelope — IF-142)

## Dependencies

- IF-065 (Deploy pre-built container images)
- IF-142 (Secret envelope — for multi-server credential transfer)
