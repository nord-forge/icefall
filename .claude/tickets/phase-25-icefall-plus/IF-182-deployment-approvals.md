# IF-182: Deployment approval gates

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Add optional approval gates for production deployments. When enabled, a deploy to the production environment requires explicit approval from an admin before executing. Provides a safety net for teams where not everyone should be able to push to production unilaterally.

## Acceptance Criteria

- [ ] Per-app setting: "Require approval for deploys" toggle (default: off)
- [ ] Optional: only require approval for specific environments (e.g., production but not staging)
- [ ] When a deploy is triggered on an approval-gated app:
  - Deploy enters `pending_approval` status
  - Notification dispatched to admins: `deploy.approval_requested`
  - Deploy card in the UI shows "Awaiting Approval" with approve/reject buttons
- [ ] Admin approves: deploy proceeds normally
- [ ] Admin rejects: deploy cancelled, reason stored, deployer notified
- [ ] Approval record: who approved/rejected, when, optional comment
- [ ] Webhook-triggered deploys also enter pending state (not auto-deployed)
- [ ] MCP tool: `approve_deploy` and `reject_deploy` tools for AI-assisted approvals
- [ ] Timeout: pending deploys auto-cancel after 24 hours if not acted on
- [ ] `deploy_approvals` table: `id`, `deploy_id`, `action` (approved/rejected), `user_id`, `comment`, `created_at`

## Technical Notes

- This modifies the deploy pipeline entry point — insert an approval check before the build starts
- The deployer role can request deploys but only admin can approve (if approval is required)
- For multi-server: approval is on the control plane, not per-server

## Dependencies

- IF-011 (Deploy pipeline — needs approval gate injection point)
- IF-043 (Notification system — approval request events)
- IF-044 (MCP server — approval tools)
