CREATE TABLE IF NOT EXISTS shared_variables (
    id TEXT PRIMARY KEY NOT NULL,
    scope TEXT NOT NULL CHECK(scope IN ('project', 'server')),
    scope_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value_encrypted BLOB NOT NULL,
    is_sensitive BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(scope, scope_id, key)
);

CREATE INDEX IF NOT EXISTS idx_shared_variables_scope ON shared_variables(scope, scope_id);
