# IF-081: Expanded database support with full backup coverage

**Phase:** 16 — v1.1 Fast Follow
**Priority:** High
**Estimate:** L

## Description

Expand the managed database provisioning system beyond the current 4 types (PostgreSQL, MySQL, Redis, MongoDB) to include MariaDB, ClickHouse, KeyDB, DragonFly, CockroachDB, SQLite, Valkey, and Cassandra. Every supported database must have a working backup/restore pipeline — no database ships without it.

## Current State

**Supported (with backups):**
- PostgreSQL 17 — `pg_dumpall` → gzip
- MySQL 8 — `mysqldump --all-databases` → gzip
- Redis 7 — `BGSAVE` + copy dump.rdb → gzip
- MongoDB 7 — `mongodump --archive --gzip`

**Missing:**
- MariaDB, ClickHouse, KeyDB, DragonFly, CockroachDB, SQLite, Valkey, Cassandra

## Acceptance Criteria

### New Database Types

#### MariaDB
- [ ] Image: `mariadb:11`
- [ ] Port: 3306
- [ ] Env vars: `MARIADB_USER`, `MARIADB_PASSWORD`, `MARIADB_DATABASE`, `MARIADB_ROOT_PASSWORD`
- [ ] Connection string: `mysql://{user}:{pass}@{host}:3306/{db}` (MySQL-compatible)
- [ ] Backup: `mariadb-dump -u {user} --all-databases | gzip`
- [ ] Restore: `gunzip | mariadb -u {user}`
- [ ] Default memory: 1024 MB

#### ClickHouse
- [ ] Image: `clickhouse/clickhouse-server:24`
- [ ] Ports: 8123 (HTTP), 9000 (native)
- [ ] Env vars: `CLICKHOUSE_USER`, `CLICKHOUSE_PASSWORD`, `CLICKHOUSE_DB`, `CLICKHOUSE_DEFAULT_ACCESS_MANAGEMENT`
- [ ] Connection string: `clickhouse://{user}:{pass}@{host}:9000/{db}`
- [ ] Backup: `clickhouse-client --query "SELECT * FROM system.tables FORMAT TabSeparated"` per table + `clickhouse-client --query "INSERT INTO ... FORMAT Native"` for data (or use `clickhouse-backup` tool)
- [ ] Alternative backup: mount data directory and use filesystem snapshot
- [ ] Default memory: 2048 MB

#### KeyDB
- [ ] Image: `eqalpha/keydb:latest`
- [ ] Port: 6379
- [ ] Env vars: none (configured via command args)
- [ ] Cmd: `keydb-server --requirepass {pass} --server-threads 2`
- [ ] Connection string: `redis://:{pass}@{host}:6379` (Redis-compatible)
- [ ] Backup: `keydb-cli BGSAVE && sleep 2 && cat /data/dump.rdb | gzip` (same as Redis)
- [ ] Default memory: 256 MB

#### DragonFly
- [ ] Image: `docker.dragonflydb.io/dragonflydb/dragonfly:latest`
- [ ] Port: 6379
- [ ] Env vars: none (configured via command args)
- [ ] Cmd: `dragonfly --requirepass {pass}`
- [ ] Connection string: `redis://:{pass}@{host}:6379` (Redis-compatible)
- [ ] Backup: `redis-cli BGSAVE && sleep 2 && cat /data/dump.rdb | gzip` (Redis-compatible)
- [ ] Default memory: 512 MB

#### CockroachDB
- [ ] Image: `cockroachdb/cockroach:latest`
- [ ] Ports: 26257 (SQL), 8080 (admin UI)
- [ ] Cmd: `start-single-node --insecure` (single-node mode for managed instances)
- [ ] Connection string: `postgresql://root@{host}:26257/defaultdb?sslmode=disable` (PostgreSQL-compatible)
- [ ] Backup: `cockroach dump --insecure --host={host} | gzip`
- [ ] Default memory: 2048 MB

#### Valkey
- [ ] Image: `valkey/valkey:8`
- [ ] Port: 6379
- [ ] Cmd: `valkey-server --requirepass {pass}`
- [ ] Connection string: `redis://:{pass}@{host}:6379` (Redis-compatible)
- [ ] Backup: `valkey-cli BGSAVE && sleep 2 && cat /data/dump.rdb | gzip` (same as Redis)
- [ ] Default memory: 256 MB

#### SQLite (managed)
- [ ] Image: `kevinko/sqlite-web:latest` or custom container with `litefs`
- [ ] Port: 8080
- [ ] Volume: `/data/db.sqlite3`
- [ ] Connection string: `file:///data/db.sqlite3`
- [ ] Backup: `sqlite3 /data/db.sqlite3 ".backup /tmp/backup.db" && gzip /tmp/backup.db`
- [ ] Default memory: 128 MB

#### Cassandra
- [ ] Image: `cassandra:5`
- [ ] Port: 9042
- [ ] Env vars: `CASSANDRA_CLUSTER_NAME`, `CASSANDRA_DC`, `CASSANDRA_RACK`
- [ ] Connection string: `cassandra://{host}:9042`
- [ ] Backup: `nodetool snapshot` + tar snapshot directory
- [ ] Default memory: 2048 MB

### Backup System Updates
- [ ] Every new database type has a corresponding entry in `backup_scheduler.rs` `dump_cmd` match
- [ ] Backup restore endpoint for each type
- [ ] Backup format documented per type
- [ ] Backup verification: after backup, check file size > 0 and is valid gzip

### Dashboard Updates
- [ ] Database creation dropdown shows all supported types
- [ ] Type-specific icons or color indicators in the database list
- [ ] Connection string format displayed correctly per type
- [ ] Backup/restore works from the UI for all types

### API Updates
- [ ] `POST /api/v1/databases` accepts all new types in the `db_type` field
- [ ] Validation rejects unknown types with a helpful error listing all supported options

## Technical Notes

- The `db_configs()` HashMap in `src/api/routes/databases.rs` is the central registry — add all new types there
- Redis-compatible databases (KeyDB, DragonFly, Valkey) share the same backup mechanism and connection string format — consider a helper
- ClickHouse and Cassandra have more complex backup strategies that may need a different approach than the simple `exec + dump` pattern
- CockroachDB is PostgreSQL wire-compatible, so the existing PostgreSQL env var name (`DATABASE_URL`) works

## Out of Scope

- Database clustering / replication (single-instance only)
- Database version upgrades (destroy and recreate)
- Custom database images (only official images)
- Database monitoring dashboards (use existing container metrics)

## Dependencies

- IF-029 (managed database provisioning), IF-030 (automated backups)
