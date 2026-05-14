# IF-159: Registration enable/disable toggle

**Phase:** 25 — Parity Gaps
**Priority:** High
**Estimate:** S

## Description

Add a settings toggle to enable or disable new user registration. When disabled, only admin-invited users can create accounts. The onboarding flow creates the first admin; after that, registration should be controllable.

## Acceptance Criteria

- [ ] New `registration_enabled` boolean in settings (default: `true`)
- [ ] `PUT /settings` accepts `registration_enabled` field (admin only)
- [ ] `POST /auth/register` returns 403 when registration is disabled
- [ ] Settings page: toggle in the "Security" or "Users" section
- [ ] When disabled, the register page shows a message: "Registration is disabled. Contact an administrator."
- [ ] Invite flow (IF-085) still works regardless of registration toggle

## Dependencies

- IF-085 (Admin user management)
