# Icefall — Project Board

> Last updated: 2026-05-14

---

## Backlog — Priority Order

**Now (Feature parity — Phase 24):**
1. IF-208..IF-219, IF-223..IF-225 — Remaining feature parity tickets

**Then (Parity gaps — Phase 25):**
8. IF-174 — GitHub App integration (largest, start early)
9. IF-159..IF-173 — Remaining parity gap tickets

**Then (Differentiators — Phase 26):**
10. IF-187 + IF-188 — Config Time Machine + Deploy Replay (small, high value)
11. IF-184 — MCP Deploy Copilot (demo value)
12. IF-183 — Ghost Mode (headline feature)

---

## In Progress

| Ticket | Title | Assignee | Started |
|--------|-------|----------|---------|
| — | — | — | — |

---

## In Review

| Ticket | Title | Reviewer | PR |
|--------|-------|----------|----|
| — | — | — | — |

---

## Done

### Phase 1 — Foundation
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-001](tickets/phase-1-foundation/IF-001-rust-project-scaffold.md) | Rust project scaffold | 2026-05-07 | Full Cargo.toml, binary crate, Clap CLI with all subcommands |
| [IF-002](tickets/phase-1-foundation/IF-002-database-schema-sqlite.md) | Database schema & SQLite setup | 2026-05-07 | 11 migrations, 40+ trait methods, AES-256-GCM encryption with tests |
| [IF-003](tickets/phase-1-foundation/IF-003-config-system.md) | Configuration system | 2026-05-07 | TOML loading, env var overrides, validation, defaults |
| [IF-004](tickets/phase-1-foundation/IF-004-docker-client.md) | Docker Engine client | 2026-05-07 | Full Bollard wrapper — containers, images, networks, volumes, stats, logs |
| [IF-005](tickets/phase-1-foundation/IF-005-caddy-client.md) | Caddy admin API client | 2026-05-07 | Route CRUD, wildcard support, operation queue with retry |
| [IF-006](tickets/phase-1-foundation/IF-006-rest-api-skeleton.md) | REST API skeleton (Axum) | 2026-05-07 | Router, middleware (tracing/CORS/request IDs), error types, all route stubs |
| [IF-007](tickets/phase-1-foundation/IF-007-daemon-lifecycle.md) | Daemon lifecycle management | 2026-05-07 | Full startup sequence, PID file, signal handling, systemd unit generation |

### Phase 2 — Build Engine
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-008](tickets/phase-2-build-engine/IF-008-framework-detection.md) | Framework detection engine | 2026-05-07 | 8 frameworks, 4 package managers, node version detection, user overrides, 20 tests |
| [IF-009](tickets/phase-2-build-engine/IF-009-dockerfile-builder.md) | Dockerfile generation per framework | 2026-05-07 | Multi-stage templates, layer caching, non-root user, Caddy/Node runtimes, 12 tests |
| [IF-010](tickets/phase-2-build-engine/IF-010-image-builder.md) | Docker image build orchestrator | 2026-05-07 | Full pipeline (clone→detect→generate→build→tag→cleanup), secret redaction, timeout, 7 tests |

### Phase 3 — Deployment Pipeline
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-011](tickets/phase-3-deployment-pipeline/IF-011-container-deploy.md) | Container deployment & lifecycle | 2026-05-07 | Zero-downtime deploy, health checks, Caddy route switching, rollback, labels, resource limits |
| [IF-012](tickets/phase-3-deployment-pipeline/IF-012-webhook-receiver.md) | Git webhook receiver | 2026-05-07 | GitHub + GitLab endpoints, HMAC-SHA256 validation, branch routing, build rate limiting |
| [IF-013](tickets/phase-3-deployment-pipeline/IF-013-preview-environments.md) | Preview environments | 2026-05-07 | Branch sanitization, glob pattern matching, auto-create/destroy, subdomain routing |
| [IF-014](tickets/phase-3-deployment-pipeline/IF-014-env-var-management.md) | Environment variable management | 2026-05-07 | Full CRUD, .env import, scope resolution, reserved keys, container restart on change |
| [IF-015](tickets/phase-3-deployment-pipeline/IF-015-sse-build-streaming.md) | SSE event streaming | 2026-05-07 | EventBus (broadcast), global/app/deploy SSE streams, reconnect via last-event-id |

### Phase 4 — Web Dashboard
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-016](tickets/phase-4-web-dashboard/IF-016-astro-project-setup.md) | Astro + Preact project setup | 2026-05-07 | Astro 6, Preact islands, CSS Modules + OKLCH tokens, sidebar, theme toggle, API/SSE utilities |
| [IF-017](tickets/phase-4-web-dashboard/IF-017-dashboard-home.md) | Dashboard home page | 2026-05-07 | Server stats bars (CPU/RAM/Disk via sysinfo), app card grid, empty state, SSE live status |
| [IF-018](tickets/phase-4-web-dashboard/IF-018-app-create-flow.md) | App creation flow | 2026-05-07 | 4-step wizard (repo → build → env vars → review), auto-name from repo, .env import |
| [IF-019](tickets/phase-4-web-dashboard/IF-019-app-detail-page.md) | App detail page | 2026-05-07 | 6 tabs (overview/deploys/logs/env/domains/settings), client-side routing, status cards |
| [IF-020](tickets/phase-4-web-dashboard/IF-020-env-var-editor.md) | Environment variable editor UI | 2026-05-07 | Masked values, click-to-reveal, scope badges, inline add, .env import dialog |
| [IF-021](tickets/phase-4-web-dashboard/IF-021-log-viewer.md) | Log viewer component | 2026-05-07 | Always-dark terminal, line numbers, level badges, search, level filter, auto-scroll, download |
| [IF-022](tickets/phase-4-web-dashboard/IF-022-deploy-view.md) | Deploy view with build steps | 2026-05-07 | Expandable build steps, streaming output via SSE, status icons, duration tracking |

### Phase 5 — Domains & Proxy
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-023](tickets/phase-5-domains-proxy/IF-023-domain-management.md) | Domain management | 2026-05-07 | DNS verification, delete + Caddy cleanup, auto IP detection, SSL provisioning |
| [IF-024](tickets/phase-5-domains-proxy/IF-024-wildcard-domain-setup.md) | Base domain & wildcard setup | 2026-05-07 | Setup + verify endpoints, Caddy wildcard config, DNS instruction generation |

### Phase 6 — Monitoring
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-025](tickets/phase-6-monitoring/IF-025-health-checks.md) | Health check system | 2026-05-07 | TCP + Docker health checks, background runner, auto-restart, SSE events, uptime calculation |
| [IF-026](tickets/phase-6-monitoring/IF-026-container-metrics.md) | Container metrics collection | 2026-05-07 | Background polling (10s), in-memory ring buffer (1h), metrics API + history endpoint |
| [IF-027](tickets/phase-6-monitoring/IF-027-log-search.md) | Log storage and search | 2026-05-07 | File-based log capture, rotation (50MB), search + filter API, download, secret redaction |
| [IF-028](tickets/phase-6-monitoring/IF-028-uptime-timeline-ui.md) | Uptime timeline UI | 2026-05-07 | 48-segment timeline bar, 24h/7d/30d range, hover tooltips, uptime percentage |
| [REVIEW](tickets/phase-6-monitoring/REVIEW-phase-6.md) | Phase 6 code review & audit | 2026-05-07 | Fixed: is_check_due interval, unbounded task spawning, uptime range scoping |

### Phase 7 — Databases
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-029](tickets/phase-7-databases/IF-029-managed-database-provisioning.md) | Managed database provisioning | 2026-05-07 | Postgres/MySQL/Redis/MongoDB, auto-credentials, volumes, connection strings, app linking |
| [IF-030](tickets/phase-7-databases/IF-030-database-backups.md) | Automated database backups | 2026-05-07 | Background scheduler, Docker exec dumps, rotation (keep 7), backup history API, manual trigger |
| [IF-031](tickets/phase-7-databases/IF-031-database-ui.md) | Database management UI | 2026-05-07 | List/create/detail pages, masked connection strings, backup section, delete with danger zone |
| [REVIEW](tickets/phase-7-databases/REVIEW-phase-7.md) | Phase 7 code review & audit | 2026-05-07 | Clippy clean, 86 tests pass |

### Phase 8 — Auth & API
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-032](tickets/phase-8-auth-api/IF-032-authentication.md) | Authentication system | 2026-05-07 | Login/logout, Argon2 hashing, session cookies, first-run admin setup, password change |
| [IF-033](tickets/phase-8-auth-api/IF-033-oauth-github-gitlab.md) | OAuth (GitHub/GitLab) | 2026-05-07 | OAuth flow endpoints ready, provider config in settings (implementation hooks) |
| [IF-034](tickets/phase-8-auth-api/IF-034-user-management.md) | User management & roles | 2026-05-07 | Invite flow with tokens, 3 roles (admin/deployer/viewer), role enforcement, deactivation with lockout protection |
| [IF-035](tickets/phase-8-auth-api/IF-035-api-tokens.md) | API token management | 2026-05-07 | `icefall_` prefixed tokens, SHA-256 hashed, CRUD API, last-used tracking, optional expiry |
| [IF-036](tickets/phase-8-auth-api/IF-036-openapi-spec.md) | OpenAPI specification | 2026-05-07 | OpenAPI 3.1 spec at /api/v1/openapi.json, all endpoints documented |
| [REVIEW](tickets/phase-8-auth-api/REVIEW-phase-8.md) | Phase 8 code review & audit | 2026-05-07 | 90 tests pass, clippy clean |

### Phase 9 — CLI
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-037](tickets/phase-9-cli/IF-037-cli-deploy-command.md) | CLI deploy command | 2026-05-07 | `icefall deploy` reads .icefall.toml, triggers deploy via API, streams status |
| [IF-038](tickets/phase-9-cli/IF-038-cli-management-commands.md) | CLI management commands | 2026-05-07 | apps list/info, env set/list, domains add/list, db create/list/backup, logs, status — all via HTTP API client |
| [IF-039](tickets/phase-9-cli/IF-039-cli-update-command.md) | CLI self-update | 2026-05-07 | Version check stub, full implementation deferred to release tooling |
| [REVIEW](tickets/phase-9-cli/REVIEW-phase-9.md) | Phase 9 code review & audit | 2026-05-07 | 90 tests pass, clippy clean |

### Phase 10 — Install & Migration
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-040](tickets/phase-10-install-migration/IF-040-install-script.md) | Installation script | 2026-05-07 | Bash install.sh: OS/arch detection, Docker+Caddy install, config gen, systemd service, idempotent |
| [IF-041](tickets/phase-10-install-migration/IF-041-server-migration-export-import.md) | Server migration | 2026-05-07 | `icefall migrate export/import`: SQLite + config + logs + backups as tar.gz, step-by-step progress |
| ~~IF-042~~ | ~~Setup wizard~~ | — | Superseded by Phase 13 |
| [REVIEW](tickets/phase-10-install-migration/REVIEW-phase-10.md) | Phase 10 code review | 2026-05-07 | 90 tests, clippy clean |

### Phase 11 — MCP & Notifications
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-043](tickets/phase-11-mcp-notifications/IF-043-notification-system.md) | Notification system | 2026-05-07 | Webhook dispatch, SMTP/Plunk stubs, channel CRUD, per-app rules, test endpoint, event types |
| [IF-044](tickets/phase-11-mcp-notifications/IF-044-mcp-server.md) | MCP server | 2026-05-07 | 13 tools (list/get apps, deploy, logs, env vars, databases, health, domains, restart), role-based permissions |
| [IF-045](tickets/phase-11-mcp-notifications/IF-045-settings-page.md) | Global settings page | 2026-05-07 | Dashboard page: domain, notifications, backup S3/R2, MCP config snippet, version display |
| [REVIEW](tickets/phase-11-mcp-notifications/REVIEW-phase-11.md) | Phase 11 code review | 2026-05-07 | 90 tests, clippy clean |

### Phase 12 — Landing Page & Documentation
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-046](tickets/phase-12-landing-docs/IF-046-landing-page.md) | Landing page | 2026-05-07 | Starlight splash page with hero, features grid, install command, tech stack |
| [IF-047](tickets/phase-12-landing-docs/IF-047-documentation-site.md) | Documentation site | 2026-05-07 | 31 doc pages across 9 sections, Pagefind search, edit links, sidebar nav |
| [IF-048](tickets/phase-12-landing-docs/IF-048-framework-guides.md) | Framework guides | 2026-05-07 | Astro + Next.js with full content, 6 more framework stubs ready for content |
| [IF-049](tickets/phase-12-landing-docs/IF-049-social-assets.md) | Brand assets | 2026-05-07 | SVG logo (light/dark), favicon, brand guidelines in assets/brand/ |
| [REVIEW](tickets/phase-12-landing-docs/REVIEW-phase-12.md) | Phase 12 review | 2026-05-07 | 34 pages build, search index, sitemap |

### Phase 13 — Onboarding
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-050](tickets/phase-13-onboarding/IF-050-onboarding-state-machine.md) | Onboarding state machine | 2026-05-07 | DB-persisted state, 6-step flow, optional/required steps, API endpoints |
| [IF-051](tickets/phase-13-onboarding/IF-051-onboarding-ui-shell.md) | Onboarding UI shell | 2026-05-07 | Dedicated layout: header with logo, centered content, step dots at bottom, no sidebar/footer |
| [IF-052](tickets/phase-13-onboarding/IF-052-step-admin-account.md) | Step 1 — Admin account | 2026-05-07 | Email + password form, Argon2 hashing, auto-session creation |
| [IF-053](tickets/phase-13-onboarding/IF-053-step-server-check.md) | Step 2 — Server check | 2026-05-07 | Docker, Caddy, disk, memory checks with pass/warn/fail status |
| [IF-054](tickets/phase-13-onboarding/IF-054-step-base-domain.md) | Step 3 — Base domain | 2026-05-07 | Domain input, DNS instructions, Caddy wildcard config, skippable |
| [IF-055](tickets/phase-13-onboarding/IF-055-step-git-provider.md) | Step 4 — Git provider | 2026-05-07 | GitHub/GitLab connect cards, skippable |
| [IF-056](tickets/phase-13-onboarding/IF-056-step-first-app.md) | Step 5 — First app | 2026-05-07 | Repo URL + app name form, creates app + triggers deploy |
| [IF-057](tickets/phase-13-onboarding/IF-057-step-first-deploy.md) | Step 6 — First deploy | 2026-05-07 | Deploy progress spinner, auto-advances on completion |
| [IF-058](tickets/phase-13-onboarding/IF-058-onboarding-completion.md) | Completion & handoff | 2026-05-07 | Success screen with checkmark, "Go to Dashboard" button |
| [REVIEW](tickets/phase-13-onboarding/REVIEW-phase-13.md) | Phase 13 review | 2026-05-07 | 90 tests, clippy clean, 6 dashboard pages |

### Phase 14 — Dashboard Surface (v1.0 Tier 1)
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-059](tickets/phase-14-dashboard-surface/IF-059-start-stop-restart-controls.md) | Start / Stop / Restart controls | 2026-05-08 | 3 API endpoints (apps + dbs), AppHeader buttons, SettingsTab danger zone controls |
| [IF-060](tickets/phase-14-dashboard-surface/IF-060-health-check-ui.md) | Health check UI | 2026-05-08 | Health panel in OverviewTab: status badge, uptime %, event history, 3-col grid |
| [IF-061](tickets/phase-14-dashboard-surface/IF-061-resource-limits-ui.md) | Resource limits UI | 2026-05-08 | Memory (MB) + CPU shares fields in SettingsTab, warning banner when unset |
| [IF-062](tickets/phase-14-dashboard-surface/IF-062-auto-deploy-webhooks-ui.md) | Auto-deploy toggle & webhook URL | 2026-05-08 | Webhook URL + secret display with copy buttons, GitHub/GitLab setup hints |
| [IF-063](tickets/phase-14-dashboard-surface/IF-063-preview-environment-ui.md) | Preview environment UI | 2026-05-08 | Enable toggle, branch glob pattern input in SettingsTab |
| [IF-064](tickets/phase-14-dashboard-surface/IF-064-persistent-storage-ui.md) | Persistent storage / volumes UI | 2026-05-08 | Volume mount CRUD (source/target/read-only), migration, SettingsTab section |

### Phase 15 — Critical Gaps (v1.0 Tier 2 + 3)
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-065](tickets/phase-15-critical-gaps/IF-065-deploy-docker-images.md) | Deploy pre-built Docker images | 2026-05-08 | New app type, Git/Image wizard choice, deploy pipeline bypass, AppHeader/OverviewTab updates |
| [IF-066](tickets/phase-15-critical-gaps/IF-066-container-rollback.md) | Container rollback | 2026-05-08 | Rollback button per deploy in DeploysTab, new API endpoint, reuses existing image |
| [IF-067](tickets/phase-15-critical-gaps/IF-067-smtp-notifications.md) | Finish SMTP notifications | 2026-05-08 | lettre crate with tokio1-rustls-tls, STARTTLS/TLS/plain modes, HTML email |
| [IF-068](tickets/phase-15-critical-gaps/IF-068-slack-discord-notifications.md) | Finish Slack + Discord notifications | 2026-05-08 | Slack Block Kit + Discord embed payloads, event-based color mapping |
| [IF-069](tickets/phase-15-critical-gaps/IF-069-path-based-routing.md) | Path-based routing | 2026-05-08 | Path field on domains, Caddy route/handle_path matcher, DomainsTab UI |
| [IF-070](tickets/phase-15-critical-gaps/IF-070-instance-backup-s3.md) | Scheduled instance backup to S3 | 2026-05-08 | Background scheduler, tar.gz archive, S3 upload, settings UI with history |
| [IF-071](tickets/phase-15-critical-gaps/IF-071-notification-subscriptions-ui.md) | Per-event notification subscriptions | 2026-05-08 | Checkbox matrix (events x channels) in Settings page |
| [IF-072](tickets/phase-15-critical-gaps/IF-072-tags.md) | App tags | 2026-05-08 | Tags column, tag input in SettingsTab, filter chips on AppGrid, tag display on AppCard |

### Phase 16 — Self-Update System
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-097](tickets/phase-16-self-update/IF-097-release-pipeline-signing.md) | Release pipeline & binary signing | 2026-05-13 | GitHub Actions multi-arch musl builds, Ed25519 manifest signing, release workflow, real keypair generated |
| [IF-098](tickets/phase-16-self-update/IF-098-update-discovery-api.md) | Update discovery & version checking | 2026-05-13 | GitHub Releases polling, manifest verification, semver comparison, API endpoints |
| [IF-099](tickets/phase-16-self-update/IF-099-update-download-verify.md) | Update download & integrity verification | 2026-05-13 | Streaming download with progress, SHA-256 + Ed25519 chain, extraction & validation |
| [IF-100](tickets/phase-16-self-update/IF-100-update-apply-restart.md) | Update apply, restart & graceful shutdown | 2026-05-13 | Atomic binary swap, SQLite backup + migrations, systemd socket activation (listenfd + sd-notify), zero-downtime, WatchdogSec=60 |
| [IF-101](tickets/phase-16-self-update/IF-101-update-rollback.md) | Update rollback & failure recovery | 2026-05-13 | Automatic rollback via ExecStopPost, systemd watchdog, manual rollback CLI/API, `rollback --check` entry point, 7-day cleanup task |
| [IF-102](tickets/phase-16-self-update/IF-102-update-dashboard-ui.md) | Update dashboard UI | 2026-05-13 | Sidebar pill, 7-step update dialog, reconnection overlay, settings section |
| [IF-103](tickets/phase-16-self-update/IF-103-auto-update-scheduling.md) | Auto-update scheduling | 2026-05-13 | Maintenance window, pre-download, deploy-aware, breaking change skip |
| [IF-104](tickets/phase-16-self-update/IF-104-cli-update-command.md) | CLI update command | 2026-05-13 | Interactive + scripted update, offline --from-file, rollback subcommand |

### Phase 17 — v1.1 Fast Follow
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-073](tickets/phase-17-v1.1-fast-follow/IF-073-docker-compose-support.md) | Docker Compose support | 2026-05-08 | Compose parser, multi-service deploy, isolated networks, variable interpolation, depends_on ordering |
| [IF-074](tickets/phase-17-v1.1-fast-follow/IF-074-projects.md) | Projects (resource grouping) | 2026-05-08 | Project CRUD, app/db assignment, sidebar grouping, settings dropdown, project detail page |
| [IF-075](tickets/phase-17-v1.1-fast-follow/IF-075-two-factor-authentication.md) | Two-Factor Authentication (2FA) | 2026-05-08 | TOTP setup with QR code, backup codes, login flow, admin reset, settings UI |
| [IF-076](tickets/phase-17-v1.1-fast-follow/IF-076-oauth-sso.md) | OAuth SSO (GitHub + Google) | 2026-05-08 | PKCE flow, account linking, provider config, login buttons, settings admin |
| [IF-077](tickets/phase-17-v1.1-fast-follow/IF-077-container-terminal.md) | Container terminal (browser shell) | 2026-05-08 | xterm.js + WebSocket + Docker exec, resize, dark theme, new tab |
| [IF-078](tickets/phase-17-v1.1-fast-follow/IF-078-command-palette.md) | Command palette | 2026-05-08 | Cmd+K fuzzy search, actions, recent items, keyboard nav, layout mount |
| [IF-079](tickets/phase-17-v1.1-fast-follow/IF-079-volume-management.md) | Volume management & browsing | 2026-05-08 | File browser drawer, upload/download, size tracking, path validation |
| [IF-080](tickets/phase-17-v1.1-fast-follow/IF-080-s3-object-storage-mounts.md) | S3 / object storage mounts | 2026-05-08 | rclone sidecar, S3 volume type in settings, shared Docker volumes |
| [IF-081](tickets/phase-17-v1.1-fast-follow/IF-081-expanded-database-support.md) | Expanded database support | 2026-05-08 | MariaDB, ClickHouse, KeyDB, DragonFly, CockroachDB, Valkey, Cassandra + backups for all |
| [IF-082](tickets/phase-17-v1.1-fast-follow/IF-082-native-static-deploy.md) | Native static site deployment | 2026-05-13 | No-Docker deploy for static sites, Caddy file_server, symlink rollback, <5s deploys, HTTP health check, deploy mode badges |
| [IF-083](tickets/phase-17-v1.1-fast-follow/IF-083-user-profile-page.md) | User profile page | 2026-05-13 | 8-section profile page: account info, password, email, 2FA, OAuth, tokens, sessions, preferences, danger zone |
| [IF-084](tickets/phase-17-v1.1-fast-follow/IF-084-user-preferences.md) | User preferences | 2026-05-13 | Theme, timezone, default project, email notification preferences; DB-persisted, API endpoints |
| [IF-085](tickets/phase-17-v1.1-fast-follow/IF-085-admin-user-management.md) | Admin user management | 2026-05-13 | Registration controls, role management, password/2FA reset, invite with SMTP email, pending invites |

### Phase 18 — UX Polish
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-086](tickets/phase-18-ux-polish/IF-086-view-transitions.md) | View Transitions (without crashes) | 2026-05-10 | ClientRouter enabled, all islands client:only, transition:persist sidebar, fade main content |
| [IF-087](tickets/phase-18-ux-polish/IF-087-prefetching.md) | Link prefetching & data preloading | 2026-05-10 | data-astro-prefetch="hover" on cross-page links, API cache layer |
| [IF-088](tickets/phase-18-ux-polish/IF-088-skeleton-loading.md) | Skeleton loading states | 2026-05-10 | SkeletonTable + SkeletonCard components with shimmer animation |
| [IF-089](tickets/phase-18-ux-polish/IF-089-app-detail-routing-polish.md) | App detail routing polish | 2026-05-10 | Lazy-loaded tab components with fade transition, AppDetailRouter |
| [IF-090](tickets/phase-18-ux-polish/IF-090-sidebar-active-state.md) | Sidebar navigation polish | 2026-05-10 | aria-current="page", active indicator, mobile drawer with backdrop, keyboard nav |
| [IF-091](tickets/phase-18-ux-polish/IF-091-toast-notifications.md) | Toast notification system | 2026-05-10 | Global toast store, success/error/info/warning types, auto-dismiss, aria-live |
| [IF-092](tickets/phase-18-ux-polish/IF-092-optimistic-updates.md) | Optimistic UI updates | 2026-05-10 | Instant deploy/rollback feedback in AppHeader + DeploysTab, revert on error |
| [IF-093](tickets/phase-18-ux-polish/IF-093-responsive-polish.md) | Responsive design polish | 2026-05-10 | Mobile drawer sidebar, responsive layouts |
| [IF-094](tickets/phase-18-ux-polish/IF-094-empty-states.md) | Empty states & onboarding hints | 2026-05-10 | Shared EmptyState component, per-page empty states with action buttons |
| [IF-095](tickets/phase-18-ux-polish/IF-095-confirmation-dialogs.md) | Consistent confirmation dialogs | 2026-05-10 | Shared ConfirmDialog with focus trap, danger/default variants, escape/backdrop close |
| [IF-096](tickets/phase-18-ux-polish/IF-096-keyboard-shortcuts.md) | Global keyboard shortcuts | 2026-05-10 | g+h/d/s/p navigation, ? help overlay, KeyboardShortcuts island |

### Phase 20A — Multi-Server Foundation
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-117](tickets/phase-20-multi-server/IF-117-database-migration-servers-table.md) | Database migration — servers table | 2026-05-11 | `servers` + `server_metrics_history` tables, `server_id` FK on apps/deploys, auto-seed control-plane record, 8 CRUD + 3 metrics trait methods |
| [IF-118](tickets/phase-20-multi-server/IF-118-server-crud-api-endpoints.md) | Server CRUD API endpoints | 2026-05-11 | POST/GET/PUT/DELETE servers, token regeneration, setup script endpoint, `Forbidden` error variant |
| [IF-119](tickets/phase-20-multi-server/IF-119-agent-websocket-endpoint.md) | Agent WebSocket endpoint on control plane | 2026-05-11 | WS auth via token hash, `AgentRegistry` with connection tracking, heartbeat checker (45s timeout), pending request map with oneshot channels, server.connected/disconnected SSE events |
| [IF-120](tickets/phase-20-multi-server/IF-120-cargo-workspace-icefall-common.md) | Cargo workspace — icefall-common crate | 2026-05-11 | Workspace with `[workspace.dependencies]`, `common/` crate with shared `AgentMessage` protocol + types, `agent/` crate skeleton, release profile (LTO + strip) |
| [IF-121](tickets/phase-20-multi-server/IF-121-agent-binary-skeleton.md) | Agent binary skeleton | 2026-05-11 | TOML config + env overrides, WS client with exponential backoff (1s→300s, ±20% jitter), 15s heartbeat/10s pong timeout, SIGTERM/SIGINT graceful shutdown, stub message handlers |
| [IF-122](tickets/phase-20-multi-server/IF-122-agent-enrollment-flow.md) | Agent enrollment flow | 2026-05-11 | `POST /agent/enroll` with 15-min TTL + single-use validation, `agt_` worker token, X25519 keypair generation, agent writes config + private key (0600) to disk |
| [IF-123](tickets/phase-20-multi-server/IF-123-install-script-control-plane.md) | Install script served by control plane | 2026-05-11 | Full bash script: Docker/Caddy auto-install, architecture detection, SHA-256 checksum verification, hardened systemd service, colored output with NO_COLOR support, `GET /agent/download/{target}` endpoint |
| [IF-124](tickets/phase-20-multi-server/IF-124-release-workflow-agent-binary.md) | Release workflow for agent binary | 2026-05-11 | Workspace builds both binaries, agent packaged as separate tarball per arch, `agent_artifacts` in signed manifest, `build.rs` embeds version/commit/target/date |

### Phase 20B — Agent Core
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-125](tickets/phase-20-multi-server/IF-125-agent-docker-operations-handler.md) | Agent Docker operations handler | 2026-05-11 | Container CRUD + inspect + list, image pull/build, volume CRUD, network create/remove via bollard |
| [IF-126](tickets/phase-20-multi-server/IF-126-agent-log-streaming.md) | Agent log streaming | 2026-05-11 | Subscribe/unsubscribe handlers, event streaming via `container.log` events |
| [IF-127](tickets/phase-20-multi-server/IF-127-agent-metrics-collection.md) | Agent metrics collection | 2026-05-11 | 10s interval, sysinfo (CPU/RAM/disk/load), per-container Docker stats, `metrics.system` + `metrics.container` events |
| [IF-128](tickets/phase-20-multi-server/IF-128-agent-health-check-execution.md) | Agent health check execution | 2026-05-11 | HTTP health checks with configurable port/path/attempts/interval/timeout, retry logic |
| [IF-129](tickets/phase-20-multi-server/IF-129-agent-terminal-proxy.md) | Agent terminal proxy | 2026-05-11 | Terminal open/input/resize/close via Docker exec TTY, base64 data, `terminal.output` events |
| [IF-130](tickets/phase-20-multi-server/IF-130-agent-caddy-management.md) | Agent Caddy management | 2026-05-11 | Caddy route add/update/remove via admin API at localhost:2019 |

### Phase 20C — Deploy Pipeline
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-131](tickets/phase-20-multi-server/IF-131-server-aware-deploy-manager.md) | Server-aware deploy manager | 2026-05-11 | `resolve_target()` + `make_remote_executor()`, remote blue-green deploy via RemoteExecutor, backward-compatible local path |
| [IF-132](tickets/phase-20-multi-server/IF-132-agent-build-pipeline.md) | Agent build pipeline | 2026-05-11 | `build.run` via RemoteExecutor, git clone + detect + Dockerfile gen + docker build on worker, streamed output |
| [IF-133](tickets/phase-20-multi-server/IF-133-worker-caddy-route-management.md) | Worker Caddy route management | 2026-05-11 | Caddy route add/update/remove delegated to agent on deploy, TLS via ACME on worker |
| [IF-134](tickets/phase-20-multi-server/IF-134-app-migration-between-servers.md) | App migration between servers | 2026-05-11 | `PUT /apps/{id}/migrate`, validation (server exists/connected/not draining), volume loss acknowledgment, 202 with migration deploy |
| [IF-135](tickets/phase-20-multi-server/IF-135-server-selection-app-creation.md) | Server selection in app creation | 2026-05-11 | `server_id` on create, validation (exists/connected/not draining), `recommendation_score` on server list, weighted composite scoring |

### Phase 20D — Dashboard UI
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-136](tickets/phase-20-multi-server/IF-136-servers-list-page.md) | Servers list page | 2026-05-11 | `/servers` page with ServerCard grid, SSE real-time status, single-server redirect, sidebar updated |
| [IF-137](tickets/phase-20-multi-server/IF-137-add-server-flow-dashboard.md) | Add server flow (dashboard) | 2026-05-11 | Inline AddServerPanel: name/host inputs, setup command with copy, token countdown, 4-step SSE enrollment progress |
| [IF-138](tickets/phase-20-multi-server/IF-138-server-detail-page.md) | Server detail page | 2026-05-11 | `/servers/[id]` with 4 tabs (Overview/Apps/Metrics/Settings), metric cards, charts, danger zone for workers |
| [IF-139](tickets/phase-20-multi-server/IF-139-app-creation-server-selection.md) | App creation server selection | 2026-05-11 | Radio-card ServerSelectStep in wizard (2+ servers only), recommendation badge, resource bars, `server_id` in create API |
| [IF-140](tickets/phase-20-multi-server/IF-140-app-detail-server-indicator.md) | App detail server indicator | 2026-05-11 | "on {server}" in AppHeader, server label on AppCard, migration UI in SettingsTab with volume-loss acknowledgment |
| [IF-141](tickets/phase-20-multi-server/IF-141-dashboard-home-aggregate-metrics.md) | Dashboard home aggregate metrics | 2026-05-11 | Weighted CPU avg, total RAM/Disk across servers, ServerHealthStrip with status dots, single-server unchanged |

### Phase 20E — Polish & Security
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-142](tickets/phase-20-multi-server/IF-142-secret-envelope-encrypted-env-vars.md) | Secret envelope encrypted env vars | 2026-05-13 | X25519 DH + AES-256-GCM envelope encryption for env vars during remote deploys, round-trip tested |
| [IF-143](tickets/phase-20-multi-server/IF-143-agent-auto-update.md) | Agent auto-update mechanism | 2026-05-13 | system.update command, agent download+verify+atomic swap, API endpoints for single/all agent updates, dashboard version badge |
| [IF-144](tickets/phase-20-multi-server/IF-144-offline-server-handling.md) | Offline server handling | 2026-05-13 | OfflineServerBanner in layout, SSE-driven nanostore, StatusDot unreachable variant, auto-dismiss on reconnect |
| [IF-145](tickets/phase-20-multi-server/IF-145-audit-logging-server-operations.md) | Audit logging for server operations | 2026-05-13 | audit_log table + migration, DB trait + SQLite impl, API endpoints (global + per-server), 90-day daily pruning |
| [IF-146](tickets/phase-20-multi-server/IF-146-setup-script-hardening.md) | Setup script hardening | 2026-05-13 | Idempotent, NO_COLOR, Alpine/OpenRC, error trap with line numbers, Docker/Caddy verification, uninstall endpoint, log file |

### Phase 22 — Expansion (v1.2)
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-147](tickets/phase-22-expansion/IF-147-environments-per-project.md) | Environments per project | — | Production/staging/dev per project, 3-level variable cascade |
| [IF-148](tickets/phase-22-expansion/IF-148-one-click-service-templates.md) | One-click service templates | — | 20 templates (Ghost, Plausible, etc.), Compose-based deploy, template browser UI |
| [IF-149](tickets/phase-22-expansion/IF-149-reverse-proxy-management-ui.md) | Reverse proxy management UI | — | Read-only Caddy viewer, middleware presets, advanced edit mode |
| [IF-150](tickets/phase-22-expansion/IF-150-log-drains.md) | Log drains | — | Grafana Loki, Axiom, generic HTTP; batched shipping, per-app + global |
| [IF-151](tickets/phase-22-expansion/IF-151-cloudflare-tunnel-integration.md) | Cloudflare Tunnel integration | — | Managed cloudflared container, per-domain tunnel routing |
| [IF-152](tickets/phase-22-expansion/IF-152-automated-docker-cleanup.md) | Automated Docker cleanup | — | Scheduled + threshold-based cleanup, deploy-aware, per-server |

### Phase 23 — Rust Quality & Performance
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-205](tickets/phase-23-rust-quality/IF-205-container-runtime-research.md) | **Container runtime research** | 2026-05-13 | Docker default + Podman opt-in recommendation |
| [IF-153](tickets/phase-23-rust-quality/IF-153-sqlite-module-split.md) | Split sqlite.rs into domain modules | 2026-05-13 | 22 files split into directory modules, 119 tests pass |
| [IF-154](tickets/phase-23-rust-quality/IF-154-large-file-splits.md) | Split remaining large Rust files | 2026-05-13 | 22 files over 400 lines → directory modules across 3 phases |
| [IF-155](tickets/phase-23-rust-quality/IF-155-performance-audit.md) | Rust performance audit | 2026-05-14 | N+1 queries, Vec pre-allocation, Arc clone elimination — PR #33 |
| [IF-156](tickets/phase-23-rust-quality/IF-156-rust-code-quality-audit.md) | Rust code quality audit | 2026-05-14 | Bare unwrap() elimination, idiomatic patterns — PR #32 |
| [IF-157](tickets/phase-23-rust-quality/IF-157-error-type-consolidation.md) | Error type consolidation | 2026-05-14 | Error type improvements — PR #35 |
| [IF-158](tickets/phase-23-rust-quality/IF-158-test-coverage-expansion.md) | Test coverage expansion | 2026-05-14 | 235 tests passing (+1519 lines), compose/native/deploy/db/env coverage |

### Phase 24 — Feature Parity
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-208](tickets/phase-24-feature-parity/IF-208-scheduled-tasks-cron-in-container.md) | Scheduled tasks (cron-in-container) | — | Cron-based tasks via container exec, execution history, manual trigger |
| [IF-209](tickets/phase-24-feature-parity/IF-209-shared-variables-hierarchical.md) | Shared variables (hierarchical) | — | Project/server-scoped env vars with cascade inheritance |
| [IF-210](tickets/phase-24-feature-parity/IF-210-environment-cloning.md) | Environment cloning | — | Clone environments/resources to other projects, servers; optional volume data copy |
| [IF-211](tickets/phase-24-feature-parity/IF-211-pre-deploy-commands.md) | Pre-deployment commands | — | Run commands in temp container before swap, fail deploy on error |
| [IF-212](tickets/phase-24-feature-parity/IF-212-http-basic-auth-per-app.md) | HTTP basic auth per app | — | Caddy basicauth directive, toggle per app for staging/internal tools |
| [IF-213](tickets/phase-24-feature-parity/IF-213-server-ssh-terminal.md) | Server-level terminal | — | In-browser host OS shell via PTY, admin-only, per-server enable |
| [IF-214](tickets/phase-24-feature-parity/IF-214-docker-cleanup-per-server.md) | Container cleanup per server | — | Per-server cleanup config, thresholds, execution history, manual trigger |
| [IF-215](tickets/phase-24-feature-parity/IF-215-database-backup-import-restore.md) | Database backup import & restore | — | Upload dump file or restore from S3, per-engine commands, chunked upload |
| [IF-216](tickets/phase-24-feature-parity/IF-216-server-disk-usage-alerts.md) | Server disk usage alerts | — | Warning/critical thresholds, hysteresis, recovery notifications |
| [IF-217](tickets/phase-24-feature-parity/IF-217-ssl-certificate-monitoring.md) | SSL certificate expiration monitoring | — | Daily cert check, expiry alerts at 14d/7d/0d, domain status indicators |
| [IF-218](tickets/phase-24-feature-parity/IF-218-global-search.md) | Global search across resources | — | Persistent search bar, unified cross-type results, keyboard nav |
| [IF-219](tickets/phase-24-feature-parity/IF-219-unmanaged-container-visibility.md) | Unmanaged container visibility | — | Show non-Icefall containers on server, basic lifecycle actions |
| [IF-220](tickets/phase-24-feature-parity/IF-220-configuration-drift-detection.md) | Configuration drift detection | 2026-05-14 | SHA-256 config hash, drift API, DriftBanner component, stores hash on deploy |
| [IF-221](tickets/phase-24-feature-parity/IF-221-force-rebuild-without-cache.md) | Force rebuild without cache | 2026-05-14 | no_cache param through full pipeline, split deploy button, CLI --no-cache, MCP |
| [IF-222](tickets/phase-24-feature-parity/IF-222-deployment-cancel.md) | Cancel in-progress deployment | 2026-05-14 | Cancel API, status check between build steps, cancel button in DeploysTab |
| [IF-223](tickets/phase-24-feature-parity/IF-223-webhook-outbound-generic.md) | Outbound webhook notifications | — | Configurable HTTP webhooks with HMAC, retry, delivery log |
| [IF-224](tickets/phase-24-feature-parity/IF-224-git-submodule-lfs-support.md) | Git submodule & LFS support | — | Per-app toggles for submodules, LFS, shallow clone |
| [IF-225](tickets/phase-24-feature-parity/IF-225-database-ssl-certificates.md) | Database SSL certificates | — | Per-server CA, per-DB certs, SSL mode config, cert rotation |

### Phase 25 — Parity Gaps
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-159](tickets/phase-25-parity-gaps/IF-159-registration-toggle.md) | Registration enable/disable | — | Settings toggle, 403 when disabled |
| [IF-160](tickets/phase-25-parity-gaps/IF-160-monorepo-base-directory.md) | Monorepo support (base directory) | — | `base_directory` field, build context subdirectory |
| [IF-161](tickets/phase-25-parity-gaps/IF-161-multiple-domains-per-app.md) | Multiple domains per app | — | Primary domain indicator, Caddy multi-route |
| [IF-162](tickets/phase-25-parity-gaps/IF-162-deploy-by-tag.md) | Deploy by git tag | — | Tag checkout, tag autocomplete, webhook tag events |
| [IF-163](tickets/phase-25-parity-gaps/IF-163-post-deploy-commands.md) | Post-deployment commands | — | Container exec after deploy, SSE streamed output |
| [IF-164](tickets/phase-25-parity-gaps/IF-164-backup-retention-config.md) | Configurable backup retention | — | Per-database retention count, replaces hardcoded 7 |
| [IF-165](tickets/phase-25-parity-gaps/IF-165-database-terminal-access.md) | Database terminal access | — | Extend IF-077 to DB containers, type-specific shells |
| [IF-166](tickets/phase-25-parity-gaps/IF-166-branch-deployment-ui.md) | Branch-specific deployment UI | — | Deploy branch field, branch autocomplete |
| [IF-167](tickets/phase-25-parity-gaps/IF-167-notification-alerts-disk-backup-server.md) | Server/disk/backup notification alerts | — | Wire 3 event types to notification dispatch |
| [IF-168](tickets/phase-25-parity-gaps/IF-168-token-ability-scoping.md) | API token ability scoping | — | Granular read/write/deploy permissions per token |
| [IF-169](tickets/phase-25-parity-gaps/IF-169-ssh-key-management.md) | SSH key management | — | Generate/import Ed25519/RSA keys, git auth integration |
| [IF-170](tickets/phase-25-parity-gaps/IF-170-docker-registry-credentials.md) | Container registry credentials | — | Registry CRUD, pull/push auth, Docker Hub/GHCR/GitLab |
| [IF-171](tickets/phase-25-parity-gaps/IF-171-internal-url-generation.md) | Internal URL generation | — | Auto `{app}.icefall.internal` hostnames for service-to-service |
| [IF-172](tickets/phase-25-parity-gaps/IF-172-public-port-tcp-proxy.md) | Public port / TCP proxy | — | Caddy L4 TCP proxy for external DB access, IP whitelist |
| [IF-173](tickets/phase-25-parity-gaps/IF-173-raw-compose-mode.md) | Raw Compose mode | — | Pass-through to docker compose / podman compose, advanced users |
| [IF-174](tickets/phase-25-parity-gaps/IF-174-github-app-integration.md) | GitHub App integration | — | Auto webhooks, PR status checks, PR comments, repo browser |
| [IF-206](tickets/phase-25-parity-gaps/IF-206-podman-runtime-support.md) | Podman runtime support (opt-in) | 2026-05-13 | Config + install detection, bollard socket swap, CI smoke tests |

### Phase 26 — Icefall+ Differentiators
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-175](tickets/phase-26-icefall-plus/IF-175-deploy-analytics-dashboard.md) | Deploy analytics dashboard | — | Frequency, success rate, build time trends, heatmap |
| [IF-176](tickets/phase-26-icefall-plus/IF-176-resource-usage-forecasting.md) | Resource usage forecasting | — | "Disk full in X days" predictions via linear regression |
| [IF-177](tickets/phase-26-icefall-plus/IF-177-deploy-preview-screenshots.md) | Deploy preview screenshots | — | Auto-capture after deploy, visual timeline |
| [IF-178](tickets/phase-26-icefall-plus/IF-178-incident-timeline.md) | Incident timeline & status page | — | Auto-detect incidents, public status page per app |
| [IF-179](tickets/phase-26-icefall-plus/IF-179-scheduled-deploys.md) | Scheduled deploys | — | Deploy at a specific time, maintenance window support |
| [IF-180](tickets/phase-26-icefall-plus/IF-180-app-dependency-graph.md) | App dependency graph | — | Interactive infrastructure visualization |
| [IF-181](tickets/phase-26-icefall-plus/IF-181-api-playground.md) | Built-in API playground | — | Interactive API explorer from OpenAPI spec |
| [IF-182](tickets/phase-26-icefall-plus/IF-182-deployment-approvals.md) | Deployment approval gates | — | Require admin approval for production deploys |
| [IF-183](tickets/phase-26-icefall-plus/IF-183-ghost-mode-hibernation.md) | Ghost Mode (container hibernation) | — | Auto-suspend idle containers, wake on first request, Rust proxy holds connection |
| [IF-184](tickets/phase-26-icefall-plus/IF-184-mcp-deploy-copilot.md) | MCP Deploy Copilot | — | Multi-step conversational deploys, diagnose, suggest_fix tools |
| [IF-185](tickets/phase-26-icefall-plus/IF-185-drift-detective.md) | Drift Detective | — | Continuous config reconciliation, detect out-of-band changes, one-click revert |
| [IF-186](tickets/phase-26-icefall-plus/IF-186-canary-probe.md) | Canary Probe | — | Post-deploy synthetic load test, auto-rollback on regression |
| [IF-187](tickets/phase-26-icefall-plus/IF-187-config-time-machine.md) | Config Time Machine | — | Full config versioning, diff any two points, one-click restore |
| [IF-188](tickets/phase-26-icefall-plus/IF-188-deploy-replay.md) | Deploy Replay | — | Structured deploy event streams, deploy diff comparison |
| [IF-189](tickets/phase-26-icefall-plus/IF-189-dead-app-detector.md) | Dead App Detector | — | Flag idle apps, suggest hibernate/delete, weekly digest |
| [IF-190](tickets/phase-26-icefall-plus/IF-190-secure-tunnel-debugger.md) | Secure Tunnel Debugger | — | `icefall tunnel` — local port forwarding via agent WebSocket, no SSH |
| [IF-191](tickets/phase-26-icefall-plus/IF-191-smart-resource-packer.md) | Smart Resource Packer | — | Right-size resource limits, co-location suggestions, savings estimates |
| [IF-192](tickets/phase-26-icefall-plus/IF-192-portable-app-bundles.md) | Portable App Bundles | — | Export/import `.icefall` files for app sharing |
| [IF-193](tickets/phase-26-icefall-plus/IF-193-noise-free-logs.md) | Noise-Free Log Streams | — | Collapse repetitive lines, anomaly highlighting, noise suppression |
| [IF-194](tickets/phase-26-icefall-plus/IF-194-power-nap-scheduler.md) | Power Nap Scheduler | — | Quiet hours: suspend low-priority apps, reduce resources for standard apps |

### Phase 27 — MCP Expansion
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-195](tickets/phase-27-mcp-expansion/IF-195-mcp-workflow-tools.md) | MCP workflow orchestration tools | — | ~30 tools: bulk ops, resource creation, server management, utilities |
| [IF-196](tickets/phase-27-mcp-expansion/IF-196-mcp-resource-protocol.md) | MCP resources & prompts protocol | — | Browsable resources (icefall://apps), pre-built prompt templates |
| [IF-197](tickets/phase-27-mcp-expansion/IF-197-mcp-claude-code-integration-guide.md) | MCP integration guides | — | Claude Code, Cursor, Windsurf setup + example workflows |

### Phase 28 — Comprehensive Documentation
| Ticket | Title | Completed | Notes |
|--------|-------|-----------|-------|
| [IF-198](tickets/phase-28-comprehensive-docs/IF-198-docs-getting-started-overhaul.md) | Getting started overhaul | — | 6 pages: intro, install, quickstart, first DB, domain, auto-deploy |
| [IF-199](tickets/phase-28-comprehensive-docs/IF-199-docs-concepts-deep-dives.md) | Concepts deep dives | — | 8 pages: architecture, builds, deploys, networking, security, multi-server, envs, databases |
| [IF-200](tickets/phase-28-comprehensive-docs/IF-200-docs-framework-guides-complete.md) | Complete framework guides | — | 19 frameworks: Astro, Next.js, Remix, SvelteKit, Laravel, Rails, Django, Go, Rust, .NET, etc. |
| [IF-201](tickets/phase-28-comprehensive-docs/IF-201-docs-how-to-guides.md) | How-to guides | — | 35+ task-oriented guides covering every common workflow |
| [IF-202](tickets/phase-28-comprehensive-docs/IF-202-docs-api-reference-complete.md) | Complete API reference | — | Every REST endpoint + MCP tool with examples, error codes, auth |
| [IF-203](tickets/phase-28-comprehensive-docs/IF-203-docs-troubleshooting-faq.md) | Troubleshooting & FAQ | — | Symptom-first troubleshooting, 6 categories + FAQ |
| [IF-204](tickets/phase-28-comprehensive-docs/IF-204-docs-migration-guides.md) | Migration guides | — | From Dokku, CapRover, Heroku, Docker Compose |
| [IF-207](tickets/phase-28-comprehensive-docs/IF-207-docs-podman-reference.md) | Podman reference docs | 2026-05-14 | Command reference, config guide, behavioral differences, setup + migration guides — PR #34 |

---

## Summary

| Metric | Count |
|--------|-------|
| Total tickets | 220 |
| Done | 152 |
| Backlog | 67 |
| Superseded | 1 |
| Phases complete | 20 / 28 |

### Progress
| Phase | Status | Tickets |
|-------|--------|---------|
| 1 — Foundation | **Done** | 7/7 |
| 2 — Build Engine | **Done** | 3/3 |
| 3 — Deployment Pipeline | **Done** | 5/5 |
| 4 — Web Dashboard | **Done** | 7/7 |
| 5 — Domains & Proxy | **Done** | 2/2 |
| 6 — Monitoring | **Done** | 5/5 |
| 7 — Databases | **Done** | 4/4 |
| 8 — Auth & API | **Done** | 6/6 |
| 9 — CLI | **Done** | 4/4 |
| 10 — Install & Migration | **Done** | 3/3 (+1 superseded) |
| 11 — MCP & Notifications | **Done** | 4/4 |
| 12 — Landing & Docs | **Done** | 5/5 |
| 13 — Onboarding | **Done** | 10/10 |
| 14 — Dashboard Surface | **Done** | 6/6 |
| 15 — Critical Gaps | **Done** | 8/8 |
| 16 — Self-Update System | **Done** | 8/8 |
| 17 — v1.1 Fast Follow | **Done** | 13/13 |
| 18 — UX Polish | **Done** | 11/11 |
| 20 — Multi-Server | **Done** | 30/30 |
| 22 — Expansion (v1.2) | Backlog | 0/6 |
| 23 — Rust Quality | **Done** | 7/7 |
| 24 — Feature Parity | **In Progress** | 3/18 |
| 25 — Parity Gaps | **In Progress** | 1/17 |
| 26 — Icefall+ | Backlog | 0/20 |
| 27 — MCP Expansion | Backlog | 0/3 |
| 28 — Comprehensive Docs | **In Progress** | 1/8 |

### Size breakdown
| Size | Count | Estimated effort |
|------|-------|-----------------|
| S | 40 | 1-2 days each |
| M | 108 | 3-5 days each |
| L | 28 | 1-2 weeks each |

### Critical path (must complete in order)
```
Phases 1-20 (done)

Expansion (Phase 22):
  IF-074 (projects) → IF-147 (environments per project)
  IF-073 (Docker Compose) → IF-148 (one-click templates)

Rust Quality (Phase 23):
  *** IF-205 (container runtime research) — DO FIRST, BLOCKS ALL PHASES 22-27 ***
  IF-205 decision → all container-touching tickets (IF-152, IF-183, IF-173, IF-165, IF-172)
  IF-153 (sqlite split) → IF-154 (remaining splits) — can run parallel with IF-205
  IF-153 → IF-155 (performance audit)
  IF-156 (quality audit) — no deps, parallel
  IF-153 → IF-157 (error consolidation)
  IF-153 + IF-157 → IF-158 (test coverage)

Feature Parity (Phase 24):
  High-value quick wins: IF-220 (drift detection), IF-221 (force rebuild), IF-222 (deploy cancel)
  IF-208 (scheduled tasks) — depends on IF-077/IF-163 (exec infrastructure)
  IF-209 (shared variables) — depends on IF-074 (projects), IF-147 (environments)
  IF-210 (env cloning) — depends on IF-147 (environments)
  IF-211 (pre-deploy) — depends on IF-163 (post-deploy, shared UI)
  IF-214 (container cleanup per server) — depends on IF-152 (base cleanup)
  IF-215 (DB import/restore) — depends on IF-030 (backups)
  IF-216 (disk alerts) — depends on IF-127 (agent metrics)
  All others are independent — can run in parallel

Parity Gaps (Phase 25):
  IF-206 (Podman support) — do after IF-205 research (done)
  All other tickets independent — can run in parallel
  IF-174 (GitHub App) is the largest, start early

Icefall+ (Phase 26):
  IF-183 (Ghost Mode) is the headline feature — start early
  IF-183 → IF-189 (Dead App Detector) → IF-194 (Power Nap)
  IF-184 (MCP Copilot) — high demo value, no deps
  IF-186 (Canary Probe) — no deps
  IF-187 (Config Time Machine) + IF-188 (Deploy Replay) — small, start first
  IF-190 (Secure Tunnel) — no deps

MCP Expansion (Phase 27):
  IF-195 (workflow tools) → IF-196 (resources + prompts) → IF-197 (integration guides)

Comprehensive Docs (Phase 28):
  IF-198 (getting started) — do first
  IF-200 (frameworks) + IF-201 (how-to) — can run in parallel
  IF-202 (API ref) — after IF-195/IF-196 (MCP expansion)
  IF-204 (migration guides) — after all features are stable
  IF-207 (Podman reference) — after IF-206 (Podman support)
```
