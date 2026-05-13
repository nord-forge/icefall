# IF-156: OAuth provider setup guide

**Phase:** 12 — Landing & Docs
**Priority:** Medium
**Estimate:** M

## Description

Step-by-step documentation for configuring GitHub and Google OAuth apps so users can enable social sign-in on their Icefall instance. Covers creating the OAuth app on each provider's developer console, obtaining client credentials, entering them in Icefall settings, and testing the flow.

## Acceptance Criteria

### GitHub OAuth App guide

- [ ] Step-by-step walkthrough of creating a GitHub OAuth App at `github.com/settings/developers`
- [ ] Correct callback URL format: `https://<your-domain>/api/v1/auth/oauth/github/callback`
- [ ] Screenshots or annotated descriptions for each field:
  - Application name
  - Homepage URL (user's Icefall domain)
  - Authorization callback URL
- [ ] How to copy Client ID and generate a Client Secret
- [ ] Where to paste credentials in Icefall: Settings > OAuth Providers > GitHub
- [ ] Toggle "Enable GitHub sign-in" and save
- [ ] Verify by signing out and using "Sign in with GitHub" on the login page
- [ ] Note: GitHub OAuth Apps vs GitHub Apps distinction (we use OAuth Apps)
- [ ] Note: organization-owned OAuth Apps and approval requirements

### Google OAuth guide

- [ ] Step-by-step walkthrough in Google Cloud Console > APIs & Services > Credentials
- [ ] Creating an OAuth 2.0 Client ID (Web application type)
- [ ] Correct authorized redirect URI: `https://<your-domain>/api/v1/auth/oauth/google/callback`
- [ ] Configuring the OAuth consent screen:
  - App name, user support email, developer contact
  - Scopes: `email`, `profile`, `openid`
  - Publishing status (testing vs production) and user cap implications
- [ ] How to copy Client ID and Client Secret
- [ ] Where to paste credentials in Icefall: Settings > OAuth Providers > Google
- [ ] Toggle "Enable Google sign-in" and save
- [ ] Verify by signing out and using "Sign in with Google" on the login page
- [ ] Note: Google Workspace domain restriction (optional)

### General

- [ ] Page lives in the Authentication section of the docs sidebar
- [ ] Cross-link from the Icefall Settings > OAuth Providers section (inline help or "View guide" link)
- [ ] Cross-link from the Profile > Connected Accounts section
- [ ] Troubleshooting subsection:
  - "redirect_uri_mismatch" error (callback URL doesn't match)
  - "access_denied" error (consent screen not published / user not added as test user)
  - OAuth works but user gets "Not configured" in Connected Accounts (provider not enabled in Settings)
- [ ] Security notes: never share client secrets, rotate if compromised

## Out of Scope

- GitLab OAuth setup (separate ticket when GitLab support ships)
- SAML/OIDC enterprise SSO
- OAuth app creation automation (API-driven setup)

## Dependencies

- IF-047 (documentation site must exist)
