CREATE TABLE IF NOT EXISTS cleanup_runs (
    id TEXT PRIMARY KEY NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    freed_bytes INTEGER NOT NULL DEFAULT 0,
    removed_items INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    details TEXT
);
