# IF-165: Database terminal access

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** S

## Description

Extend the container terminal (IF-077) to work with managed database containers. Users can open a shell into their database container for debugging, running ad-hoc queries, or inspecting the filesystem.

## Acceptance Criteria

- [ ] "Terminal" button on the database detail page
- [ ] Opens the same xterm.js terminal as IF-077, targeting the database container
- [ ] Default shell command varies by database type:
  - PostgreSQL: `psql -U postgres`
  - MySQL/MariaDB: `mysql -u root -p`
  - MongoDB: `mongosh`
  - Redis/Valkey/KeyDB: `redis-cli`
  - ClickHouse: `clickhouse-client`
  - Others: `/bin/sh`
- [ ] Role enforcement: admin and deployer can access, viewer cannot
- [ ] Warning banner: "You are connected to a live database. Commands execute immediately."
- [ ] For multi-server: terminal proxied through the agent (IF-129)

## Technical Notes

- Reuse the entire terminal infrastructure from IF-077 — just pass the database container ID instead of app container ID
- The database container ID is stored on the managed_databases record

## Dependencies

- IF-077 (Container terminal)
- IF-129 (Agent terminal proxy — for multi-server)
