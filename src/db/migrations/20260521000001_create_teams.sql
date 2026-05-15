-- Phase 30: Teams & Multi-Tenancy

CREATE TABLE IF NOT EXISTS teams (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    owner_id TEXT NOT NULL REFERENCES users(id),
    settings TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS team_memberships (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member', 'viewer')),
    invited_by TEXT REFERENCES users(id),
    accepted_at TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(team_id, user_id)
);

CREATE TABLE IF NOT EXISTS team_invitations (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('admin', 'member', 'viewer')),
    token TEXT NOT NULL UNIQUE,
    invited_by TEXT NOT NULL REFERENCES users(id),
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Add team_id to resource tables (nullable for backward compat during migration)
ALTER TABLE apps ADD COLUMN team_id TEXT REFERENCES teams(id);
ALTER TABLE projects ADD COLUMN team_id TEXT REFERENCES teams(id);
ALTER TABLE databases ADD COLUMN team_id TEXT REFERENCES teams(id);
ALTER TABLE ssh_keys ADD COLUMN team_id TEXT REFERENCES teams(id);
ALTER TABLE registries ADD COLUMN team_id TEXT REFERENCES teams(id);
ALTER TABLE notifications ADD COLUMN team_id TEXT REFERENCES teams(id);
ALTER TABLE api_tokens ADD COLUMN team_id TEXT REFERENCES teams(id);

-- Add active_team_id to sessions for team context
ALTER TABLE sessions ADD COLUMN active_team_id TEXT REFERENCES teams(id);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_team_memberships_user ON team_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_team_memberships_team ON team_memberships(team_id);
CREATE INDEX IF NOT EXISTS idx_team_invitations_email ON team_invitations(email);
CREATE INDEX IF NOT EXISTS idx_team_invitations_token ON team_invitations(token);
CREATE INDEX IF NOT EXISTS idx_apps_team ON apps(team_id);
CREATE INDEX IF NOT EXISTS idx_projects_team ON projects(team_id);
CREATE INDEX IF NOT EXISTS idx_databases_team ON databases(team_id);
