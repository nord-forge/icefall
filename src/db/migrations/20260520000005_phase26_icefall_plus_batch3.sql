-- Phase 26 batch 3: remaining Icefall+ features

-- IF-183: Ghost Mode — container hibernation
ALTER TABLE apps ADD COLUMN ghost_mode_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE apps ADD COLUMN ghost_mode_idle_minutes INTEGER NOT NULL DEFAULT 30;
ALTER TABLE apps ADD COLUMN ghost_mode_status TEXT NOT NULL DEFAULT 'active';

-- IF-178: Incident timeline
CREATE TABLE IF NOT EXISTS incidents (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'investigating' CHECK (status IN ('investigating', 'identified', 'monitoring', 'resolved')),
    severity TEXT NOT NULL DEFAULT 'minor' CHECK (severity IN ('minor', 'major', 'critical')),
    affected_apps TEXT,
    affected_servers TEXT,
    root_cause TEXT,
    started_at TEXT NOT NULL,
    resolved_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS incident_notes (
    id TEXT PRIMARY KEY NOT NULL,
    incident_id TEXT NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    author_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_incidents_status ON incidents(status);
CREATE INDEX IF NOT EXISTS idx_incident_notes_incident ON incident_notes(incident_id);

-- IF-178: Public status page opt-in
ALTER TABLE apps ADD COLUMN status_page_enabled BOOLEAN NOT NULL DEFAULT FALSE;

-- IF-191: Smart Resource Packer — uses existing metrics, no new tables needed

-- IF-194: Power Nap Scheduler
ALTER TABLE apps ADD COLUMN power_nap_priority TEXT NOT NULL DEFAULT 'standard';
ALTER TABLE apps ADD COLUMN power_nap_custom_schedule TEXT;

-- IF-177: Deploy preview screenshots path tracking
ALTER TABLE deploys ADD COLUMN screenshot_path TEXT;
