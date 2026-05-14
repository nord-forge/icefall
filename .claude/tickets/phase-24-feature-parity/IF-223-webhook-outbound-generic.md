# IF-223: Outbound webhook notifications (generic HTTP)

**Phase:** 24 — Feature Parity
**Priority:** Low
**Estimate:** S

## Description

IF-043 already supports webhook notifications, but this ticket ensures the webhook channel supports the full generic HTTP webhook pattern: configurable URL, HTTP method, custom headers, request body template, and retry with backoff. This makes Icefall's notification system extensible to any HTTP-based service without needing dedicated integrations.

## Acceptance Criteria

- [ ] Webhook channel configuration: URL, HTTP method (POST/PUT, default POST), custom headers (key-value pairs), optional secret for HMAC signature
- [ ] Request body: JSON payload with event type, resource info, timestamp, and message
- [ ] HMAC-SHA256 signature header (`X-Icefall-Signature`) when secret is configured
- [ ] Retry: 3 attempts with exponential backoff (1s, 5s, 25s) on 5xx or network errors
- [ ] Timeout: 10 seconds per attempt
- [ ] Multiple webhook endpoints per instance (not just one)
- [ ] Test button: sends a test payload and shows the response status
- [ ] Webhook delivery log: last 50 deliveries with status code, response time, and retry count
- [ ] Failed delivery status visible in notification settings
- [ ] API: webhook channel CRUD at `GET/POST /notifications/webhooks`, `PUT/DELETE /notifications/webhooks/{id}`

## Technical Notes

- The existing webhook dispatch in IF-043 is a good foundation — this extends it with configurability and observability
- Store delivery history in `webhook_deliveries` table for debugging
- Consider a `webhook_templates` concept: pre-format payloads for common targets (PagerDuty, Opsgenie, custom)

## Out of Scope

- Inbound webhooks (already covered by IF-012/IF-062)
- GraphQL webhook targets
- Webhook transformation / scripting (Lua, JS)

## Dependencies

- IF-043 (Notification system — base webhook dispatch)
