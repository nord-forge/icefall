# FE-254: Team onboarding flow

**Phase:** 30
**Priority:** Medium
**Size:** S
**Dependencies:** FE-253

## Description

Update the onboarding wizard to create a team during initial setup, and handle invitation acceptance for new users.

## Changes

- Onboarding step: "Name your team" (after creating account, before first app)
- Invitation accept page: `/invitations/{token}` — shows team name, role, accept/decline buttons
- If invited user doesn't have an account, redirect to registration with invitation context

## Acceptance Criteria

- Given a new install, when the first user completes onboarding, then a team is created with their chosen name
- Given an invitation link, when a new user clicks it, then they register and join the team in one flow

## Out of Scope

Team templates, onboarding checklists per team
