-- Phase 30: Cross-team server sharing

CREATE TABLE IF NOT EXISTS server_team_access (
    id TEXT PRIMARY KEY NOT NULL,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    access_level TEXT NOT NULL CHECK(access_level IN ('deploy', 'read-only')),
    granted_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL,
    UNIQUE(server_id, team_id)
);

CREATE INDEX IF NOT EXISTS idx_server_team_access_server ON server_team_access(server_id);
CREATE INDEX IF NOT EXISTS idx_server_team_access_team ON server_team_access(team_id);
