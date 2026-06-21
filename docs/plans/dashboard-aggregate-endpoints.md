# Dashboard aggregate endpoints — future backend work

The new dashboard widgets (`AttentionRow`, `Housekeeping`) currently fan out
**one request per app / per server / per database** from the browser, because
no server-side aggregate exists. This is bounded with `mapLimit` (see
`@lib/concurrency`) and fails per-item silently, so it's safe — but on a large
install (hundreds of apps) it means a burst of requests on every dashboard load.

A future backend PR should add aggregate endpoints so each widget needs **one**
request. Suggested shapes (all under `/api/v1/`):

## 1. Health summary — replaces per-app `GET /apps/{id}/health`
Used by: `AttentionRow` (failing health checks)

```
GET /apps/health/summary
→ { data: { unhealthy_count: number, total_checks: number,
            apps: [{ app_id, app_name, unhealthy_checks }] } }
```

## 2. Capacity forecast summary — replaces per-server `GET /servers/{id}/forecast`
Used by: `AttentionRow` (disk/memory full forecast)

```
GET /servers/forecast/summary?warn_days=14
→ { data: [{ server_id, server_name, resource: 'disk'|'memory',
             days_until_full }] }   // only servers under the threshold
```

## 3. Optimization summary — replaces per-server `GET /servers/{id}/optimizations`
Used by: `Housekeeping` (optimization tips)

```
GET /servers/optimizations/summary
→ { data: { count: number, ram_saved_bytes: number,
            estimated_monthly_savings_usd: number } }
```

## 4. Backup freshness summary — replaces per-db `GET /databases/{id}/backups`
Used by: `Housekeeping` (stale backups)

```
GET /databases/backups/summary?stale_days=7
→ { data: { total: number, stale_count: number,
            stale: [{ database_id, name, last_backup_at }] } }
```

## Migration
Once these land, swap the `mapLimit(...)` blocks in `AttentionRow.tsx` and
`Housekeeping.tsx` for a single `request()` to the summary endpoint and delete
the now-unused fan-out. The `@lib/concurrency` helper can stay (it's generic) or
be removed if nothing else uses it.
