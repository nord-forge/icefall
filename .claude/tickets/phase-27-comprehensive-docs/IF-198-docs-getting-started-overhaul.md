# IF-198: Getting Started docs overhaul

**Phase:** 27 — Comprehensive Docs
**Priority:** High
**Estimate:** M

## Description

Rewrite the getting started section to be the best onboarding experience in the PaaS space. Every step should have copy-pasteable commands, expected output, screenshots, and troubleshooting tips. A new user should go from zero to deployed app in under 10 minutes following these docs.

## Pages to Create/Rewrite

### `getting-started/introduction.mdx` (rewrite)
- [ ] What is Icefall — 2 sentences, not a paragraph
- [ ] Who it's for (solo devs, small teams, 1-10 apps)
- [ ] How it's different (single binary, SQLite, MCP, agent architecture)
- [ ] System requirements table (OS, RAM, disk, Docker version)
- [ ] Architecture diagram (single-server vs multi-server)

### `getting-started/installation.mdx` (rewrite)
- [ ] One-liner install command with explanation of each flag
- [ ] What the installer does (step by step with expected output)
- [ ] Manual install alternative (for users who don't trust curl | bash)
- [ ] Platform-specific notes: Ubuntu, Debian, CentOS, Alpine, Arch
- [ ] Docker and Caddy version requirements
- [ ] Firewall ports to open (80, 443, 8443 dashboard)
- [ ] Post-install verification checklist

### `getting-started/quick-start.mdx` (rewrite)
- [ ] 5-minute quickstart: install → onboarding wizard → deploy first app
- [ ] Screenshots of every onboarding step
- [ ] "Deploy from GitHub" walkthrough (most common path)
- [ ] "Deploy a Docker image" walkthrough (second most common)
- [ ] Expected result: your app is live at `https://your-app.your-domain.com`
- [ ] Next steps: add a database, set up auto-deploy, configure a domain

### `getting-started/first-database.mdx` (new)
- [ ] Create a PostgreSQL database
- [ ] Link it to your app
- [ ] Use the connection string in your app's env vars
- [ ] Test the connection from the database browser
- [ ] Set up automatic backups

### `getting-started/custom-domain.mdx` (new)
- [ ] Add a custom domain
- [ ] DNS configuration (A record, CNAME)
- [ ] SSL certificate provisioning (automatic via Caddy)
- [ ] Verify it's working
- [ ] Troubleshooting: DNS propagation, certificate errors

### `getting-started/auto-deploy.mdx` (new)
- [ ] Enable auto-deploy on push
- [ ] Copy the webhook URL
- [ ] Configure in GitHub/GitLab (step by step with screenshots)
- [ ] Test with a push
- [ ] Preview environments setup

## Podman Parity

Every getting started page that mentions Docker must include Podman equivalents:
- [ ] Installation page: Docker install AND Podman install paths side by side
- [ ] All `docker` CLI commands shown with `podman` equivalent in a tabbed code block
- [ ] Quick start: detect runtime section explaining auto-detection during install
- [ ] Troubleshooting: Podman-specific issues (socket not active, version too old, networking)

## Standards

- [ ] Every command includes expected output
- [ ] Every UI step includes a screenshot
- [ ] Every page has a "Troubleshooting" section at the bottom
- [ ] Every page has "Next" and "Previous" navigation
- [ ] Code blocks use proper language tags and are copy-pasteable
- [ ] Docker/Podman commands shown in tabbed blocks (tab: Docker / tab: Podman)

## Dependencies

- IF-047 (Documentation site — existing Starlight setup)
- IF-206 (Podman runtime support)
