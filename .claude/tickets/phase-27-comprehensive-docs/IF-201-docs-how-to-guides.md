# IF-201: How-to guides for common workflows

**Phase:** 27 — Comprehensive Docs
**Priority:** High
**Estimate:** L

## Description

Write task-oriented how-to guides for every common workflow a user might need. These are action guides ("How do I...") as opposed to concept docs ("How does it work"). Each guide should be completeable in 5-15 minutes.

## Guides to Create

### Deployment Workflows
- [ ] `guides/deploy-from-github.mdx` — deploy a GitHub repo (webhook setup, auto-deploy)
- [ ] `guides/deploy-from-gitlab.mdx` — deploy a GitLab repo (webhook setup)
- [ ] `guides/deploy-docker-image.mdx` — deploy a pre-built Docker image from a registry
- [ ] `guides/deploy-docker-compose.mdx` — deploy a multi-service Compose stack
- [ ] `guides/deploy-static-site.mdx` — deploy a static site (native, no Docker)
- [ ] `guides/deploy-monorepo.mdx` — deploy from a monorepo subdirectory
- [ ] `guides/preview-environments.mdx` — set up PR-based preview deploys
- [ ] `guides/rollback-deploy.mdx` — roll back to a previous version
- [ ] `guides/scheduled-deploy.mdx` — schedule a deploy for a specific time

### Database Operations
- [ ] `guides/connect-database.mdx` — create a DB, link to app, use the connection string
- [ ] `guides/backup-restore.mdx` — set up backups, download a backup, restore
- [ ] `guides/database-browser.mdx` — run queries from the dashboard
- [ ] `guides/external-database.mdx` — connect to an external database (not managed by Icefall)

### Domain & Networking
- [ ] `guides/custom-domain.mdx` — add and verify a custom domain
- [ ] `guides/wildcard-domain.mdx` — set up a base domain with wildcard subdomains
- [ ] `guides/cloudflare-tunnel.mdx` — deploy behind Cloudflare Tunnel (no public IP)
- [ ] `guides/path-routing.mdx` — route `/api` and `/` to different services

### Multi-Server
- [ ] `guides/add-server.mdx` — add a worker server to your Icefall instance
- [ ] `guides/migrate-app.mdx` — move an app from one server to another
- [ ] `guides/server-selection.mdx` — how server recommendation scoring works

### Security & Auth
- [ ] `guides/setup-2fa.mdx` — enable TOTP two-factor authentication
- [ ] `guides/oauth-github.mdx` — configure GitHub OAuth login
- [ ] `guides/oauth-google.mdx` — configure Google OAuth login
- [ ] `guides/api-tokens.mdx` — create and use API tokens
- [ ] `guides/invite-users.mdx` — invite team members with role assignment

### Monitoring & Notifications
- [ ] `guides/health-checks.mdx` — configure health checks for an app
- [ ] `guides/setup-notifications.mdx` — configure Slack/Discord/SMTP/webhook notifications
- [ ] `guides/log-search.mdx` — search and filter application logs
- [ ] `guides/log-drains.mdx` — stream logs to Grafana Loki or Axiom

### MCP & CLI
- [ ] `guides/mcp-claude-code.mdx` — manage Icefall from Claude Code
- [ ] `guides/mcp-cursor.mdx` — manage Icefall from Cursor
- [ ] `guides/cli-deploy.mdx` — deploy from the command line
- [ ] `guides/cli-management.mdx` — manage apps, databases, and domains via CLI

### Advanced
- [ ] `guides/self-update.mdx` — update Icefall to the latest version
- [ ] `guides/server-migration.mdx` — migrate your entire Icefall instance to a new server
- [ ] `guides/environment-variables.mdx` — manage env vars with scoping and .env import
- [ ] `guides/resource-limits.mdx` — configure CPU and memory limits
- [ ] `guides/ghost-mode.mdx` — set up container hibernation for cost savings
- [ ] `guides/custom-dockerfile.mdx` — override auto-detected Dockerfile

## Standards

- [ ] Each guide: title, prerequisites, numbered steps, expected results, troubleshooting
- [ ] Code blocks are complete and copy-pasteable
- [ ] Screenshots for every UI interaction
- [ ] "Time to complete" estimate at the top of each guide
- [ ] Cross-links to related concept docs and other guides

## Dependencies

- IF-047 (Documentation site)
