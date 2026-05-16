# FE-253: Team management UI

**Phase:** 30
**Priority:** High
**Size:** L
**Dependencies:** BE-250, BE-251

## Description

Dashboard pages for team management.

## Create

- `dashboard/src/islands/teams/TeamsPage/TeamsPage.tsx` — list of user's teams with create button
- `dashboard/src/islands/teams/TeamDetail/TeamDetail.tsx` — team settings, member list, invitation management
- `dashboard/src/islands/teams/TeamSwitcher/TeamSwitcher.tsx` — dropdown in sidebar header to switch active team
- `dashboard/src/islands/teams/InviteModal/InviteModal.tsx` — email + role picker for inviting members

## Modify

- Sidebar layout — add TeamSwitcher above navigation
- Settings page — add team management link
- Profile page — show "My Teams" section

## Acceptance Criteria

- Given a user with multiple teams, when they click the team switcher, then they can switch and the dashboard reloads with scoped data
- Given a team admin, when they open team detail, then they can see members, change roles, and send invitations
- Given a team viewer, when they open team detail, then they can see members but cannot manage them

## Out of Scope

Team billing, usage analytics per team
