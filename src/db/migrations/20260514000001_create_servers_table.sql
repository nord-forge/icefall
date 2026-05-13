-- Multi-server support: servers table, metrics history, and FK columns on apps/deploys

CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('control-plane', 'worker')),
    status TEXT NOT NULL DEFAULT 'online' CHECK (status IN ('online', 'offline', 'enrolling', 'draining')),
    token_hash TEXT,
    agent_version TEXT,
    labels TEXT,
    resources TEXT,
    public_key TEXT,
    last_heartbeat_at TEXT,
    registered_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS server_metrics_history (
    id TEXT PRIMARY KEY NOT NULL,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    cpu_percent REAL,
    ram_used_bytes INTEGER,
    ram_total_bytes INTEGER,
    disk_used_bytes INTEGER,
    disk_total_bytes INTEGER,
    load_average TEXT,
    recorded_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_server_metrics_history_server_time
    ON server_metrics_history(server_id, recorded_at);

ALTER TABLE apps ADD COLUMN server_id TEXT REFERENCES servers(id);
ALTER TABLE deploys ADD COLUMN server_id TEXT REFERENCES servers(id);

-- Auto-seed the control-plane server record
INSERT INTO servers (id, name, host, role, status, created_at, updated_at)
VALUES (
    'cp_ctrl_0000000001',
    'Control Plane',
    'localhost',
    'control-plane',
    'online',
    datetime('now'),
    datetime('now')
);

-- Backfill existing apps and deploys to the control-plane server
UPDATE apps SET server_id = 'cp_ctrl_0000000001' WHERE server_id IS NULL;
UPDATE deploys SET server_id = 'cp_ctrl_0000000001' WHERE server_id IS NULL;
