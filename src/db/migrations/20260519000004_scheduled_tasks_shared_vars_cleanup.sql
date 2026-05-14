-- Phase 24 batch 4: IF-208 (scheduled tasks), IF-209 (shared variables), IF-214 (container cleanup per server)

-- IF-208: Scheduled tasks
CREATE TABLE IF NOT EXISTS scheduled_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    command TEXT NOT NULL,
    cron_expression TEXT NOT NULL,
    timeout_seconds INTEGER NOT NULL DEFAULT 300,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    container_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_app_id ON scheduled_tasks(app_id);

CREATE TABLE IF NOT EXISTS scheduled_task_executions (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES scheduled_tasks(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'failed', 'timed_out')),
    output TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_task_executions_task_id ON scheduled_task_executions(task_id);

-- IF-209: Shared variables (hierarchical)
CREATE TABLE IF NOT EXISTS shared_variables (
    id TEXT PRIMARY KEY NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('project', 'server')),
    scope_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    is_sensitive BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_shared_variables_scope ON shared_variables(scope, scope_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_shared_variables_unique_key ON shared_variables(scope, scope_id, key);

-- IF-214: Container cleanup per server
ALTER TABLE servers ADD COLUMN container_cleanup_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE servers ADD COLUMN container_cleanup_frequency TEXT NOT NULL DEFAULT '0 */6 * * *';
ALTER TABLE servers ADD COLUMN container_cleanup_threshold INTEGER NOT NULL DEFAULT 80;
ALTER TABLE servers ADD COLUMN cleanup_unused_images BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE servers ADD COLUMN cleanup_unused_volumes BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE servers ADD COLUMN cleanup_unused_networks BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE servers ADD COLUMN cleanup_dangling_only BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE servers ADD COLUMN force_container_cleanup BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS container_cleanup_executions (
    id TEXT PRIMARY KEY NOT NULL,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    space_reclaimed_bytes INTEGER,
    images_removed INTEGER NOT NULL DEFAULT 0,
    volumes_removed INTEGER NOT NULL DEFAULT 0,
    networks_removed INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'failed', 'skipped'))
);

CREATE INDEX IF NOT EXISTS idx_cleanup_executions_server ON container_cleanup_executions(server_id);
