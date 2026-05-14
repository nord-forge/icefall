-- Phase 26 batch 1: Icefall+ differentiators

-- IF-187: Config Time Machine — configuration versioning
CREATE TABLE IF NOT EXISTS config_history (
    id TEXT PRIMARY KEY NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    field TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by TEXT,
    changed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_config_history_resource ON config_history(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_config_history_changed ON config_history(changed_at);

-- IF-188: Deploy Replay — structured deploy events
CREATE TABLE IF NOT EXISTS deploy_events (
    id TEXT PRIMARY KEY NOT NULL,
    deploy_id TEXT NOT NULL REFERENCES deploys(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    data TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_deploy_events_deploy ON deploy_events(deploy_id);

-- IF-189: Dead App Detector — request tracking and inactivity
ALTER TABLE apps ADD COLUMN last_request_at TEXT;
ALTER TABLE apps ADD COLUMN exempt_from_inactivity BOOLEAN NOT NULL DEFAULT FALSE;

-- IF-179: Scheduled deploys
ALTER TABLE deploys ADD COLUMN scheduled_at TEXT;

-- IF-182: Deployment approval gates
ALTER TABLE apps ADD COLUMN require_deploy_approval BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS deploy_approvals (
    id TEXT PRIMARY KEY NOT NULL,
    deploy_id TEXT NOT NULL REFERENCES deploys(id) ON DELETE CASCADE,
    action TEXT NOT NULL CHECK (action IN ('approved', 'rejected')),
    user_id TEXT NOT NULL,
    comment TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_deploy_approvals_deploy ON deploy_approvals(deploy_id);
