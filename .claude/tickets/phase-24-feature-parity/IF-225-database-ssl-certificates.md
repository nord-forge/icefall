# IF-225: Database SSL certificate management

**Phase:** 24 — Feature Parity
**Priority:** Low
**Estimate:** M

## Description

Support SSL/TLS connections for managed databases. When databases are exposed publicly (IF-172) or when compliance requires encrypted connections even on internal networks, databases need SSL certificates. This includes auto-generating self-signed CA certificates per server and per-database certificates signed by that CA.

## Acceptance Criteria

- [ ] Per-server: auto-generate a self-signed CA certificate on first database creation (stored on server)
- [ ] Per-database toggle: "Enable SSL" in database settings
- [ ] When enabled: generate a server certificate signed by the server's CA
- [ ] Certificate details shown in database settings: issuer, valid until, fingerprint
- [ ] "Regenerate Certificate" button (for rotation)
- [ ] Connection string updates to include `sslmode=require` (PostgreSQL) or equivalent per engine
- [ ] CA certificate downloadable from the UI (for clients that need to trust it)
- [ ] SSL mode selector per engine:
  - PostgreSQL: `disable`, `allow`, `prefer`, `require`, `verify-ca`, `verify-full`
  - MySQL/MariaDB: `DISABLED`, `PREFERRED`, `REQUIRED`
  - MongoDB: TLS toggle
  - Redis: TLS toggle
- [ ] For multi-server: CA and certs managed on the server where the database runs (via agent)
- [ ] API: `PUT /databases/{id}` accepts `ssl_enabled`, `ssl_mode`; `GET /databases/{id}/certificate` returns CA cert

## Technical Notes

- Use `rcgen` crate for certificate generation (already in Rust ecosystem)
- CA cert stored in `/etc/icefall/ca/` on the server (or agent data dir)
- Mount the cert files into the database container via volume mounts
- Each engine has different SSL configuration: PostgreSQL via `ssl_cert_file`/`ssl_key_file`, MySQL via `--ssl-cert`/`--ssl-key`, etc.

## Out of Scope

- Let's Encrypt certificates for databases (overkill for internal use)
- Client certificate authentication (mTLS)
- Certificate rotation automation (manual regenerate is sufficient for v1)

## Dependencies

- IF-029 (Managed database provisioning)
- IF-172 (Public port / TCP proxy — SSL especially important for public exposure)
