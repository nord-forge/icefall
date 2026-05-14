# IF-217: SSL certificate expiration monitoring

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

Caddy handles SSL certificate provisioning and renewal automatically, but renewals can fail silently (DNS issues, rate limits, ACME challenge failures). This ticket adds SSL certificate monitoring: track expiration dates for all configured domains and alert users when certificates are approaching expiry or have failed to renew.

## Acceptance Criteria

- [ ] Background job: check SSL certificate expiry for all app domains (daily, configurable)
- [ ] Certificate check via TLS handshake to each domain (read the `notAfter` from the server cert)
- [ ] Track per-domain: `domain`, `issuer`, `expires_at`, `last_checked_at`, `status` (valid/expiring/expired/error)
- [ ] Notification dispatch: `ssl.expiring` at 14 days, `ssl.critical` at 7 days, `ssl.expired` at 0 days
- [ ] Domain management page: show certificate status + expiry date per domain
- [ ] Domains list: visual indicator (green lock / amber warning / red expired)
- [ ] Event types added to notification subscription matrix (IF-071)
- [ ] API endpoint: `GET /domains/{id}/certificate` returns cert details
- [ ] For multi-server: certificate check runs on the control plane (connects to the domain's public endpoint)

## Technical Notes

- Use `rustls` or `native-tls` to perform TLS handshake and extract certificate info
- Connect to the domain on port 443, read the leaf certificate's `notAfter`
- Cache results in a `domain_certificates` table or inline on the domains table
- Don't check domains that have no DNS (pre-verification domains)

## Out of Scope

- Manual certificate upload (Caddy manages certs)
- Certificate authority selection (Caddy defaults to Let's Encrypt / ZeroSSL)
- Client certificate / mTLS management
- Database SSL certificate monitoring (separate concern)

## Dependencies

- IF-023 (Domain management — domain list)
- IF-043 (Notification system)
