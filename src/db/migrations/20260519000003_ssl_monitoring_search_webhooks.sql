-- Phase 24 batch 3: IF-217 (SSL monitoring), IF-223 (webhook enhancements), IF-219 (unmanaged containers)

-- IF-217: SSL certificate monitoring fields on domains
ALTER TABLE domains ADD COLUMN ssl_issuer TEXT;
ALTER TABLE domains ADD COLUMN ssl_expires_at TEXT;
ALTER TABLE domains ADD COLUMN ssl_last_checked_at TEXT;

-- IF-223: Enhanced webhook configuration with multiple endpoints, HMAC, retry, delivery log
CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    method TEXT NOT NULL DEFAULT 'POST',
    secret TEXT,
    headers TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    endpoint_id TEXT NOT NULL REFERENCES webhook_endpoints(id) ON DELETE CASCADE,
    event TEXT NOT NULL,
    status_code INTEGER,
    response_time_ms INTEGER,
    attempt INTEGER NOT NULL DEFAULT 1,
    error TEXT,
    delivered_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_endpoint ON webhook_deliveries(endpoint_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_delivered ON webhook_deliveries(delivered_at);
