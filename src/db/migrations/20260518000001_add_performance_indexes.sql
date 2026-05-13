-- Performance indexes for hot-path queries

-- Auth lookups (every authenticated API request)
CREATE INDEX IF NOT EXISTS idx_api_tokens_token_hash ON api_tokens(token_hash);

-- Agent connection auth
CREATE INDEX IF NOT EXISTS idx_servers_token_hash ON servers(token_hash);

-- Multi-server app filtering
CREATE INDEX IF NOT EXISTS idx_apps_server_id ON apps(server_id);

-- Multi-server deploy filtering
CREATE INDEX IF NOT EXISTS idx_deploys_server_id ON deploys(server_id);

-- Health check event queries (by check + time range)
CREATE INDEX IF NOT EXISTS idx_health_check_events_composite
    ON health_check_events(health_check_id, checked_at DESC);

-- Session lookup by expiry (for pruning)
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Deploy pruning by age
CREATE INDEX IF NOT EXISTS idx_deploys_created_at ON deploys(created_at);

-- Audit log pruning by age
CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON audit_log(created_at);
