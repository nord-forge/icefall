-- IF-179: Scheduled deploys.
--
-- The `scheduled_at` column already exists on `deploys` (baseline schema), but
-- the status CHECK constraint did not permit the two new lifecycle states a
-- scheduled deploy needs: 'scheduled' (queued, awaiting its trigger time) and
-- 'missed' (the trigger window passed while the server was offline).
--
-- SQLite cannot ALTER a CHECK constraint in place, so the table is recreated.
-- `legacy_alter_table=ON` keeps the RENAME from rewriting the foreign-key
-- references in child tables (deploy_events, deploy_approvals, canary_results),
-- which continue to point at "deploys" the whole way through. The old table is
-- then dropped — nothing references "deploys_old", so no cascade fires.

PRAGMA legacy_alter_table=ON;

ALTER TABLE deploys RENAME TO deploys_old;

CREATE TABLE deploys (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    server_id TEXT REFERENCES servers(id),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('scheduled', 'pending', 'building', 'deploying', 'running', 'failed', 'stopped', 'cancelled', 'missed')),
    git_sha TEXT,
    tag TEXT,
    build_log TEXT,
    image_ref TEXT,
    container_id TEXT,
    env_snapshot TEXT,
    config_hash TEXT,
    no_cache BOOLEAN NOT NULL DEFAULT FALSE,
    screenshot_path TEXT,
    scheduled_at TEXT,
    started_at TEXT,
    finished_at TEXT,
    created_at TEXT NOT NULL
);

INSERT INTO deploys (
    id, app_id, environment_id, server_id, status, git_sha, tag, build_log,
    image_ref, container_id, env_snapshot, config_hash, no_cache,
    screenshot_path, scheduled_at, started_at, finished_at, created_at
)
SELECT
    id, app_id, environment_id, server_id, status, git_sha, tag, build_log,
    image_ref, container_id, env_snapshot, config_hash, no_cache,
    screenshot_path, scheduled_at, started_at, finished_at, created_at
FROM deploys_old;

DROP TABLE deploys_old;

CREATE INDEX idx_deploys_app_id ON deploys(app_id);
CREATE INDEX idx_deploys_status ON deploys(status);
CREATE INDEX idx_deploys_server_id ON deploys(server_id);
CREATE INDEX idx_deploys_created_at ON deploys(created_at);
-- Drives the scheduler's "due" query: WHERE status = 'scheduled' AND scheduled_at <= now.
CREATE INDEX idx_deploys_scheduled_at ON deploys(scheduled_at) WHERE scheduled_at IS NOT NULL;

PRAGMA legacy_alter_table=OFF;
