# IF-164: Configurable backup retention count

**Phase:** 25 — Parity Gaps
**Priority:** Low
**Estimate:** S

## Description

Replace the hardcoded `keep 7` backup rotation with a user-configurable retention count per database. Users with limited disk space may want fewer backups; users with critical data may want more.

## Acceptance Criteria

- [ ] `backup_retention_count` integer field on the `managed_databases` table (default: 7, min: 1, max: 100)
- [ ] Database detail page: retention count input in the backup settings section
- [ ] Backup rotation logic uses the per-database retention count instead of hardcoded 7
- [ ] Global default in Settings page: "Default backup retention" (applies to new databases)
- [ ] Changing retention count triggers immediate cleanup if current count exceeds new limit

## Dependencies

- IF-030 (Database backups)
