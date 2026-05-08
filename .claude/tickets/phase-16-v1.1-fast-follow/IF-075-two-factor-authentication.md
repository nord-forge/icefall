# IF-075: Two-Factor Authentication (2FA)

**Phase:** 16 — v1.1 Fast Follow
**Priority:** High
**Estimate:** M

## Description

Add TOTP-based two-factor authentication with backup codes. Most Icefall users self-host behind a VPN, but users exposing their dashboard publicly need a second factor. No TOTP code exists in the codebase — this is a full implementation.

## Acceptance Criteria

### 2FA Setup Flow
- [ ] Profile/account page: "Enable Two-Factor Authentication" button
- [ ] Setup wizard:
  1. Generate TOTP secret (RFC 6238, SHA1, 30-second period, 6-digit codes)
  2. Display QR code (otpauth:// URI) scannable by any authenticator app
  3. Display secret key as text for manual entry
  4. Require user to enter a valid code to confirm setup
  5. Generate and display 10 backup codes (one-time use)
  6. Require user to confirm they've saved backup codes
- [ ] After setup: 2FA is immediately active for the user

### Login Flow
- [ ] After successful email/password login, if 2FA is enabled:
  - Show 2FA code input page (not the dashboard)
  - Accept 6-digit TOTP code from authenticator app
  - Accept backup code as alternative (mark used backup codes)
  - "Remember this device for 30 days" checkbox (optional, stores device token)
- [ ] Failed 2FA attempts: rate limit after 5 failures (5-minute lockout)
- [ ] Session is only created after successful 2FA verification

### 2FA Management
- [ ] Profile page: show 2FA status (enabled/disabled)
- [ ] "Regenerate backup codes" button (invalidates old codes, shows new ones)
- [ ] "Disable 2FA" requires entering a current TOTP code or backup code
- [ ] Admin can reset another user's 2FA (for lockout recovery)
- [ ] CLI command: `icefall reset-2fa --email user@example.com` for emergency recovery

### Backend
- [ ] Add to users table: `totp_secret` (encrypted, nullable), `totp_enabled` (boolean), `totp_backup_codes` (encrypted JSON array)
- [ ] TOTP validation: accept current code and one previous/next code (30-second window tolerance)
- [ ] Backup codes: 10 random 8-character alphanumeric codes, stored hashed (Argon2)
- [ ] Device remember tokens: stored in `sessions` table with 30-day expiry
- [ ] API endpoints:
  - `POST /api/v1/auth/2fa/setup` — generate secret and QR code
  - `POST /api/v1/auth/2fa/verify` — verify code and activate 2FA
  - `POST /api/v1/auth/2fa/validate` — validate code during login
  - `POST /api/v1/auth/2fa/backup-codes` — regenerate backup codes
  - `DELETE /api/v1/auth/2fa` — disable 2FA (requires code)
  - `DELETE /api/v1/users/{id}/2fa` — admin reset (requires admin role)

### General
- [ ] Light and dark theme verified
- [ ] Mobile responsive (QR code must be scannable on all screen sizes)

## Technical Notes

- Use `totp-rs` crate for TOTP generation and validation
- Use `qrcode` crate for QR code generation (SVG output for the dashboard)
- TOTP secret stored encrypted with AES-256-GCM (same as other secrets)
- Backup codes should be displayed ONCE during setup — not retrievable after
- Consider the `data-encoding` crate for Base32 encoding of the secret

## Out of Scope

- WebAuthn / hardware keys (FIDO2) — TOTP only for v1.1
- SMS-based 2FA
- Enforcing 2FA for all users (admin policy) — optional per-user only
- Recovery via email

## Dependencies

- IF-032 (authentication system)
