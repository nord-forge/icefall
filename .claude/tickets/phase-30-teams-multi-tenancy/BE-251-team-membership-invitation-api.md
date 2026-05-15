# BE-251: Team membership and invitation API

**Phase:** 30
**Priority:** High
**Size:** M
**Dependencies:** BE-250

## Description

Invite users to teams, manage roles, handle invitation acceptance.

## Endpoints

- `POST /teams/{id}/invite` — send invitation (email, role) — admin+ only
- `GET /teams/{id}/members` — list members with roles
- `PUT /teams/{id}/members/{user_id}` — change member role — admin+ only
- `DELETE /teams/{id}/members/{user_id}` — remove member — admin+ only (cannot remove owner)
- `POST /invitations/{token}/accept` — accept invitation (creates membership)
- `DELETE /invitations/{token}` — decline/revoke invitation

## Invitation Flow

1. Admin invites email@example.com with role "member"
2. If user exists: notification sent, can accept from UI
3. If user doesn't exist: registration link with invitation token embedded
4. Token expires after 7 days

## Acceptance Criteria

- Given an admin invites user@example.com, when the user clicks accept, then they join the team with the invited role
- Given a team owner, when they try to remove themselves, then 400 is returned
- Given an expired invitation token, when used, then 400 "Invitation expired" is returned

## Out of Scope

Email delivery (uses existing notification system), SSO group sync
