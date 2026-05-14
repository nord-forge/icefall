-- Phase 26 batch 2: Icefall+ differentiators

-- IF-186: Canary Probe — post-deploy regression detection
ALTER TABLE apps ADD COLUMN canary_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE apps ADD COLUMN canary_config TEXT;

CREATE TABLE IF NOT EXISTS canary_results (
    id TEXT PRIMARY KEY NOT NULL,
    deploy_id TEXT NOT NULL REFERENCES deploys(id) ON DELETE CASCADE,
    p50_ms REAL,
    p95_ms REAL,
    p99_ms REAL,
    error_count INTEGER NOT NULL DEFAULT 0,
    total_requests INTEGER NOT NULL DEFAULT 0,
    verdict TEXT NOT NULL CHECK (verdict IN ('pass', 'fail', 'baseline')),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_canary_results_deploy ON canary_results(deploy_id);

-- IF-185: Drift Detective — continuous reconciliation
ALTER TABLE apps ADD COLUMN drift_monitoring_enabled BOOLEAN NOT NULL DEFAULT TRUE;

CREATE TABLE IF NOT EXISTS drift_events (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    drifted_fields TEXT NOT NULL,
    declared_state TEXT,
    actual_state TEXT,
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    detected_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_drift_events_app ON drift_events(app_id);

-- IF-193: Noise-Free Logs — per-app custom patterns
ALTER TABLE apps ADD COLUMN log_noise_patterns TEXT;
ALTER TABLE apps ADD COLUMN log_highlight_patterns TEXT;

-- IF-192: Portable App Bundles — schema for bundle metadata (bundles are JSON files, no table needed for export)
-- Import tracking for auditing
CREATE TABLE IF NOT EXISTS bundle_imports (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL,
    bundle_version TEXT NOT NULL,
    source_name TEXT,
    imported_by TEXT,
    imported_at TEXT NOT NULL
);
