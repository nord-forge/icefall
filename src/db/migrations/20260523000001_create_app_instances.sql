-- Phase 31: Load Balancing — multi-instance app model

CREATE TABLE IF NOT EXISTS app_instances (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    server_id TEXT NOT NULL REFERENCES servers(id),
    status TEXT NOT NULL DEFAULT 'deploying'
        CHECK (status IN ('deploying', 'running', 'unhealthy', 'stopped', 'failed')),
    container_id TEXT,
    host_port INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_app_instances_app ON app_instances(app_id);
CREATE INDEX IF NOT EXISTS idx_app_instances_server ON app_instances(server_id);

-- desired_instances: how many instances the user wants running
ALTER TABLE apps ADD COLUMN desired_instances INTEGER NOT NULL DEFAULT 1;

-- lb_policy: load balancing policy across instances
ALTER TABLE apps ADD COLUMN lb_policy TEXT NOT NULL DEFAULT 'round_robin'
    CHECK (lb_policy IN ('round_robin', 'least_conn', 'ip_hash', 'random'));

-- lb_health_check_path: path Caddy uses for active health checks on instances
ALTER TABLE apps ADD COLUMN lb_health_check_path TEXT NOT NULL DEFAULT '/';

-- lb_sticky_sessions: pin a client to one upstream via a cookie
ALTER TABLE apps ADD COLUMN lb_sticky_sessions INTEGER NOT NULL DEFAULT 0;

-- Backfill: one app_instance row per app that has a deployed container.
-- An app's most recent successful deploy carries its container_id and server.
INSERT INTO app_instances (id, app_id, server_id, status, container_id, created_at, updated_at)
SELECT
    lower(hex(randomblob(10))),
    a.id,
    COALESCE(a.server_id, 'cp_ctrl_0000000001'),
    'running',
    d.container_id,
    datetime('now'),
    datetime('now')
FROM apps a
JOIN deploys d ON d.id = (
    SELECT id FROM deploys
    WHERE app_id = a.id AND container_id IS NOT NULL AND status = 'running'
    ORDER BY created_at DESC
    LIMIT 1
);
