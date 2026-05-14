# IF-243: Git source integration management UI

**Phase:** 29 — Frontend UI
**Priority:** High
**Estimate:** L

## Description

Build the UI for managing GitHub App integrations (IF-174). One-click connect flow, repo browser, webhook auto-setup.

## Acceptance Criteria

- [ ] Settings page: "Git Integrations" section
- [ ] "Connect GitHub" button → GitHub App installation flow
- [ ] Installation list: account, repos accessible, status
- [ ] App creation: browse repos from connected GitHub App (no manual URL)
- [ ] Branch selector populated from GitHub API
- [ ] Automatic webhook setup on repo connect
- [ ] PR status checks shown on deploy detail
- [ ] Disconnect/remove integration
- [ ] a11y: repo browser keyboard navigable, status indicators labeled

## Dependencies

- IF-174 (GitHub App backend)
