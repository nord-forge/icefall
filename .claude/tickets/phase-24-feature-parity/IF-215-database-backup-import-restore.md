# IF-215: Database backup import & restore from file

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** M

## Description

IF-030 covers automated scheduled backups and IF-070 covers S3 backup storage. This ticket adds the ability to import/restore a database from an uploaded file or from an S3-stored backup. Users need this when migrating databases between servers, restoring from a disaster, or seeding a staging database from production.

## Acceptance Criteria

- [ ] Database detail page: new "Import" tab (alongside existing backup management)
- [ ] File upload: chunked upload for large dump files (up to 2GB)
- [ ] Per-engine restore command auto-generation:
  - PostgreSQL: `pg_restore` / `psql` (detect format: custom vs plain SQL)
  - MySQL: `mysql` CLI import
  - MariaDB: `mariadb` CLI import
  - MongoDB: `mongorestore --gzip --archive`
  - Redis: `redis-cli --pipe` or RDB file restore
- [ ] S3 restore: pick from existing S3-stored backups, download and restore
- [ ] Custom restore command input (override auto-generated command)
- [ ] Restore runs via container exec (Docker/Podman) in the database container
- [ ] Live progress feedback via SSE (streaming restore output)
- [ ] Confirmation dialog: "This will overwrite the current database. Are you sure?"
- [ ] API endpoint: `POST /databases/{id}/restore` (multipart file upload or S3 reference)
- [ ] Restore history in backup executions table (type: "restore")
- [ ] For multi-server: restore executed on the server running the database container (via agent)

## Technical Notes

- Chunked upload: use multipart form with resumable chunks (tus protocol or custom chunked endpoint)
- Copy uploaded file into database container via container cp (Docker/Podman), then exec the restore command
- Clean up uploaded file from container after restore completes
- Large file handling: stream to disk, don't buffer in memory

## Out of Scope

- Point-in-time recovery (PITR)
- Cross-engine migration (e.g., MySQL dump → PostgreSQL)
- Automated production-to-staging sync

## Dependencies

- IF-029 (Managed database provisioning)
- IF-030 (Database backups — shared UI patterns)
- IF-070 (S3 backup storage — for S3 restore source)
