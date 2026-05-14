-- Phase 24: IF-221 (force rebuild), IF-222 (deploy cancel), IF-220 (config drift)
-- Adds: apps.disable_build_cache, deploys.no_cache, deploys.config_hash, 'cancelled' status

ALTER TABLE apps ADD COLUMN disable_build_cache BOOLEAN NOT NULL DEFAULT FALSE;

-- SQLite cannot modify CHECK constraints, so we recreate the deploys table.

CREATE TABLE deploys_new (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'building', 'deploying', 'running', 'failed', 'stopped', 'cancelled')),
    git_sha TEXT,
    build_log TEXT,
    started_at TEXT,
    finished_at TEXT,
    image_ref TEXT,
    container_id TEXT,
    env_snapshot TEXT,
    server_id TEXT REFERENCES servers(id),
    no_cache BOOLEAN NOT NULL DEFAULT FALSE,
    config_hash TEXT,
    created_at TEXT NOT NULL
);

INSERT INTO deploys_new (
    id, app_id, environment_id, status, git_sha, build_log,
    started_at, finished_at, image_ref, container_id, env_snapshot,
    server_id, no_cache, config_hash, created_at
)
SELECT
    id, app_id, environment_id, status, git_sha, build_log,
    started_at, finished_at, image_ref, container_id, env_snapshot,
    server_id, FALSE, NULL, created_at
FROM deploys;

DROP TABLE deploys;
ALTER TABLE deploys_new RENAME TO deploys;

CREATE INDEX IF NOT EXISTS idx_deploys_app_id ON deploys(app_id);
CREATE INDEX IF NOT EXISTS idx_deploys_status ON deploys(status);
CREATE INDEX IF NOT EXISTS idx_deploys_server_id ON deploys(server_id);
CREATE INDEX IF NOT EXISTS idx_deploys_created_at ON deploys(created_at);
