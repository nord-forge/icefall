# IF-174: GitHub App integration

**Phase:** 25 — Parity Gaps
**Priority:** High
**Estimate:** L

## Description

Build a first-party GitHub App integration for deep GitHub features: automatic webhook setup, deploy status checks on PRs, PR comments with deploy URLs, and repo browsing without manual webhook configuration. This is the single most impactful CI/CD improvement — it replaces manual webhook setup with a one-click "Install GitHub App" flow.

## Acceptance Criteria

### GitHub App Setup
- [ ] Icefall ships a GitHub App manifest that users can install on their GitHub org/account
- [ ] Settings page: "GitHub Integration" section with "Install GitHub App" button
- [ ] OAuth callback receives the installation ID and stores it
- [ ] `github_installations` table: `id`, `installation_id`, `account_login`, `account_type` (user/org), `access_token` (encrypted), `token_expires_at`, `created_at`

### Automatic Webhook Setup
- [ ] When connecting a repo: Icefall automatically creates the webhook via the GitHub API (no manual URL copying)
- [ ] Webhook events subscribed: `push`, `pull_request`, `create` (for tags)
- [ ] Webhook secret generated and stored automatically

### Deploy Status Checks
- [ ] On deploy start: create a GitHub commit status `pending` on the commit SHA
- [ ] On deploy success: update to `success` with a link to the deploy in Icefall
- [ ] On deploy failure: update to `failure` with error summary
- [ ] Status context: `icefall/deploy`

### PR Comments
- [ ] On preview environment deploy (IF-013): post a PR comment with the preview URL
- [ ] Comment updated on each subsequent push to the PR (edit, not new comment)
- [ ] On PR close/merge: comment updated with "Preview environment destroyed"
- [ ] Comment format: deploy status, preview URL, deploy duration

### Repo Browser
- [ ] App creation: browse repos and branches from the GitHub API (no manual URL entry)
- [ ] Repo list filtered by the GitHub App installation's accessible repos
- [ ] Branch selector populated from the GitHub API

### Token Refresh
- [ ] GitHub App installation tokens expire after 1 hour — auto-refresh before expiry
- [ ] Background task checks token expiry every 30 minutes

## Technical Notes

- GitHub App authentication: JWT signed with the App's private key → exchange for installation token
- Use `octocrab` crate for GitHub API interactions
- The App manifest can be auto-generated or provided as a JSON file users paste into GitHub
- PR comments use the `issues` API (PRs are issues in GitHub's model)
- Rate limiting: GitHub allows 5000 requests/hour per installation — more than enough

## Out of Scope

- GitLab equivalent (separate ticket — different API, different app model)
- Bitbucket / Gitea / Forgejo integrations
- GitHub Actions integration (users can curl the deploy API from GHA already)
- Code review features (Icefall is a PaaS, not a CI system)

## Dependencies

- IF-012 (Webhook receiver — existing manual webhook approach, this replaces/enhances it)
- IF-013 (Preview environments — for PR comments)
- IF-076 (OAuth SSO — GitHub OAuth can share the App's OAuth flow)
