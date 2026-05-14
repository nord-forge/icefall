-- Phase 24 batch 5: IF-210 (env cloning), IF-213 (server terminal), IF-215 (db restore), IF-225 (db SSL)

-- IF-213: Server terminal enable toggle
ALTER TABLE servers ADD COLUMN terminal_enabled BOOLEAN NOT NULL DEFAULT FALSE;

-- IF-225: Database SSL certificate fields
ALTER TABLE databases ADD COLUMN ssl_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE databases ADD COLUMN ssl_mode TEXT;
ALTER TABLE databases ADD COLUMN ssl_ca_cert TEXT;
ALTER TABLE databases ADD COLUMN ssl_cert TEXT;
ALTER TABLE databases ADD COLUMN ssl_key TEXT;
ALTER TABLE databases ADD COLUMN ssl_expires_at TEXT;

-- IF-215: Database restore history
CREATE TABLE IF NOT EXISTS database_restore_history (
    id TEXT PRIMARY KEY NOT NULL,
    database_id TEXT NOT NULL REFERENCES databases(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL CHECK (source_type IN ('file', 's3')),
    source_ref TEXT,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'failed')),
    output TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_restore_history_db ON database_restore_history(database_id);
