# IF-150: Log drains

**Phase:** 22 — Expansion (v1.2)
**Priority:** Medium
**Estimate:** L

## Description

Stream application logs to external logging services in real time. Logs are shipped directly from the Icefall daemon (or agent on worker servers) to the configured destination using HTTP-based protocols. Start with three drain types: Grafana Loki, Axiom, and a generic HTTP endpoint that covers everything else (Datadog, Logflare, custom collectors).

## Acceptance Criteria

### Database

- [ ] New `log_drains` table: `id`, `app_id` (FK, nullable — null = global drain), `name`, `drain_type` (enum: `loki`, `axiom`, `http`), `config` (encrypted JSON), `enabled` (boolean), `last_sent_at`, `error_count`, `last_error`, `created_at`, `updated_at`
- [ ] A global drain (app_id = null) receives logs from all apps
- [ ] An app-level drain receives logs only from that app
- [ ] An app can have multiple drains active simultaneously
- [ ] Maximum 5 drains per app + 3 global drains

### Drain Types

#### Grafana Loki
- [ ] Config: `{ "url": "https://loki.example.com", "tenant_id": "optional", "username": "optional", "password": "optional", "labels": { "env": "production" } }`
- [ ] Ships logs using Loki's push API (`POST /loki/api/v1/push`)
- [ ] Each log entry includes labels: `app` (app name), `server` (server name), `deploy_id`, `stream` (stdout/stderr), plus user-configured labels
- [ ] Batches up to 100 log lines or 1 second, whichever comes first
- [ ] Timestamps in nanosecond Unix epoch format (Loki requirement)

#### Axiom
- [ ] Config: `{ "dataset": "icefall-logs", "api_token": "xaat-...", "org_id": "optional" }`
- [ ] Ships logs using Axiom's ingest API (`POST /v1/datasets/{dataset}/ingest`)
- [ ] JSON payload with `_time`, `app`, `server`, `deploy_id`, `level`, `message` fields
- [ ] Batches up to 100 entries or 1 second

#### Generic HTTP
- [ ] Config: `{ "url": "https://...", "method": "POST", "headers": { "Authorization": "Bearer ..." }, "format": "json_lines|json_array|text", "template": "optional" }`
- [ ] `json_lines`: one JSON object per line (NDJSON) — compatible with Datadog, Logflare, most collectors
- [ ] `json_array`: array of JSON objects in a single POST
- [ ] `text`: raw log lines, one per line
- [ ] Optional `template` field for custom JSON structure using `{{message}}`, `{{level}}`, `{{app}}`, `{{timestamp}}` placeholders
- [ ] Batches up to 100 entries or 1 second

### Shipping Engine

- [ ] Background task per drain that reads from the existing log stream (reuse the SSE/log capture infrastructure)
- [ ] In-memory ring buffer (1000 entries) per drain to absorb bursts
- [ ] Retry with exponential backoff: 1s, 2s, 4s, 8s, max 60s
- [ ] After 10 consecutive failures: disable the drain, set `last_error`, emit a notification event (`log_drain.failed`)
- [ ] Drain does not block the main log pipeline — if the buffer is full, oldest entries are dropped (lossy, not lossless)
- [ ] Graceful shutdown: flush pending batches with a 5-second timeout before exit
- [ ] For multi-server: the agent ships logs for apps on its server. Global drains are shipped by the control plane (which aggregates via SSE from agents).

### API Endpoints

- [ ] `GET /apps/{id}/log-drains` — list drains for an app
- [ ] `POST /apps/{id}/log-drains` — create a drain for an app
- [ ] `GET /log-drains` — list global drains (admin only)
- [ ] `POST /log-drains` — create a global drain (admin only)
- [ ] `GET /log-drains/{id}` — get drain detail (config values masked except URL)
- [ ] `PUT /log-drains/{id}` — update drain config
- [ ] `DELETE /log-drains/{id}` — delete drain
- [ ] `POST /log-drains/{id}/test` — send a test log entry to verify configuration
- [ ] `POST /log-drains/{id}/enable` / `POST /log-drains/{id}/disable` — toggle drain

### Dashboard UI

- [ ] New "Log Drains" section in app detail Logs tab (below the log viewer)
- [ ] Drain list: name, type icon, status (active/disabled/error), last sent timestamp
- [ ] Add drain form: drain type selector (Loki / Axiom / HTTP), type-specific config fields
- [ ] "Test Connection" button that sends a test entry and shows success/failure
- [ ] Error indicator with last error message when a drain is in failed state
- [ ] "Re-enable" button for disabled drains (resets error count)
- [ ] Global drains section in Settings page (admin only)
- [ ] Drain config fields: URL and token fields are password-type inputs with reveal toggle

## Technical Notes

- The existing `LogCapture` system (IF-027) already writes logs to files and provides a stream — hook into this stream rather than reading files
- Use `tokio::sync::broadcast` to fan out log entries to multiple drain tasks
- HTTP client: use `reqwest` with connection pooling — one client per drain type
- Loki push format requires protobuf by default but accepts JSON via `Content-Type: application/json` — use JSON to avoid the protobuf dependency
- Axiom SDK exists for Rust but a simple HTTP client is lighter — use raw HTTP
- The `config` column stores encrypted JSON using the existing AES-256-GCM encryption (IF-002) since it contains API tokens

## Out of Scope

- Log aggregation across multiple apps into a single view
- Log-based alerting (use the notification system for that)
- Structured logging transformation (parsing unstructured logs into structured format)
- S3/object storage log archival (different from streaming)
- Syslog protocol support
- FluentBit/Fluentd sidecar deployment (direct HTTP shipping is simpler)

## Dependencies

- IF-027 (Log storage and search — provides the log stream to hook into)
- IF-043 (Notification system — for drain failure alerts)
- IF-127 (Agent metrics collection — for multi-server log shipping)
