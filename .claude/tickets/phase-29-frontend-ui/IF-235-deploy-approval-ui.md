# IF-235: Deploy approval workflow UI

**Phase:** 29 — Frontend UI
**Priority:** Medium
**Estimate:** S

## Description

Surface the deploy approval gate (IF-182) in the deploy flow.

## Acceptance Criteria

- [ ] App settings: "Require deploy approval" toggle
- [ ] When approval required: deploy enters "Awaiting Approval" status
- [ ] Deploy card: approve/reject buttons for admins
- [ ] Approval/rejection with optional comment
- [ ] Deploy history shows approval record (who, when, comment)
- [ ] Notification banner for pending approvals
- [ ] a11y: approval buttons clearly labeled with deploy context

## Dependencies

- IF-182 (Deploy approvals backend)
