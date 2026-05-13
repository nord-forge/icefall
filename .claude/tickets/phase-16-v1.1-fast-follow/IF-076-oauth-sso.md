# IF-076: OAuth SSO (GitHub + Google)

**Phase:** 16 — v1.1 Fast Follow
**Priority:** Medium
**Estimate:** L

## Description

Add OAuth 2.0 SSO login via GitHub and Google. Zero OAuth code exists in the backend — this is a full implementation including PKCE flows, token handling, and account linking. GitHub is the primary target (most developer users), Google provides the broadest coverage.

## Acceptance Criteria

### Login Page
- [ ] "Sign in with GitHub" button
- [ ] "Sign in with Google" button
- [ ] OAuth buttons shown below the existing email/password form
- [ ] Visual separator: "or" divider between password and OAuth sections

### OAuth Flow
- [ ] Authorization Code flow with PKCE (Proof Key for Code Exchange)
- [ ] Flow:
  1. User clicks "Sign in with GitHub/Google"
  2. Redirect to provider's authorization URL with PKCE challenge
  3. User authorizes on provider's site
  4. Redirect back to Icefall with authorization code
  5. Exchange code for access token (server-side)
  6. Fetch user profile (email, name, avatar)
  7. Create session and redirect to dashboard

### Account Linking
- [ ] First OAuth login with unknown email: create new user account (if registration is enabled)
- [ ] First OAuth login with existing email: link OAuth identity to existing account (after email verification)
- [ ] Subsequent OAuth logins: match by provider + provider user ID
- [ ] User can link multiple OAuth providers to one account
- [ ] User can unlink an OAuth provider (if they have password auth or another provider linked)

### Settings Page — OAuth Configuration
- [ ] Admin settings: OAuth provider configuration
- [ ] Per provider (GitHub, Google):
  - Client ID
  - Client Secret (stored encrypted)
  - Enabled/disabled toggle
- [ ] GitHub: callback URL display: `https://{base_domain}/api/v1/auth/oauth/github/callback`
- [ ] Google: callback URL display: `https://{base_domain}/api/v1/auth/oauth/google/callback`
- [ ] Setup instructions for each provider (where to create the OAuth app)

### Profile Page
- [ ] Show linked OAuth providers
- [ ] "Link GitHub" / "Link Google" buttons for unlinked providers
- [ ] "Unlink" button per linked provider (disabled if it's the only auth method)

### Backend
- [ ] New table: `oauth_identities` — `id`, `user_id`, `provider`, `provider_user_id`, `provider_email`, `access_token` (encrypted), `refresh_token` (encrypted), `created_at`
- [ ] API endpoints:
  - `GET /api/v1/auth/oauth/{provider}/authorize` — redirect to provider
  - `GET /api/v1/auth/oauth/{provider}/callback` — handle callback, create session
  - `POST /api/v1/auth/oauth/{provider}/link` — link provider to existing account
  - `DELETE /api/v1/auth/oauth/{provider}/unlink` — unlink provider
- [ ] Token refresh: automatically refresh expired access tokens using refresh tokens
- [ ] CSRF protection: state parameter in OAuth flow

### Providers
- [ ] **GitHub:**
  - Scopes: `read:user`, `user:email`
  - Fetch: name, email, avatar URL
  - Authorize URL: `https://github.com/login/oauth/authorize`
  - Token URL: `https://github.com/login/oauth/access_token`
  - User API: `https://api.github.com/user`
- [ ] **Google:**
  - Scopes: `openid`, `email`, `profile`
  - Fetch: name, email, avatar URL
  - Authorize URL: `https://accounts.google.com/o/oauth2/v2/auth`
  - Token URL: `https://oauth2.googleapis.com/token`
  - User info: `https://www.googleapis.com/oauth2/v2/userinfo`

### General
- [ ] Light and dark theme verified
- [ ] Mobile responsive

## Technical Notes

- Use `oauth2` crate for the OAuth 2.0 flow (supports PKCE out of the box)
- Use `reqwest` (already a dependency) for token exchange and profile fetching
- Store OAuth tokens encrypted with AES-256-GCM
- PKCE code verifier should be stored in a short-lived server-side session (not cookies)
- The onboarding flow (IF-055) already has git provider connect cards — reuse that UI pattern

## Out of Scope

- GitLab, Azure, Bitbucket OAuth (can be added later following the same pattern)
- SAML / OIDC enterprise SSO
- Requiring OAuth for all users (admin policy)
- Using OAuth tokens to access provider APIs (e.g., listing repos via GitHub token)

## Dependencies

- IF-032 (authentication), IF-075 (2FA — 2FA should work with OAuth accounts too)
