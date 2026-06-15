-- IF-191: Smart Resource Packer
--
-- Per-container CPU/memory samples, persisted so the recommendations engine can
-- analyze 7 days of usage (peak/average) to right-size limits. Until now only
-- server-level metrics were persisted; per-container stats lived in memory
-- (~1h, lost on restart), which is not enough for right-sizing.
--
-- One row per container sample. `app_id` ties the sample to its app for
-- per-app aggregation; `memory_limit_bytes` is captured alongside usage so the
-- engine can compute headroom without re-reading the live container config.

CREATE TABLE container_metrics_history (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    cpu_percent REAL NOT NULL,
    memory_usage_bytes INTEGER NOT NULL,
    memory_limit_bytes INTEGER NOT NULL,
    recorded_at TEXT NOT NULL
);

-- The hot query is "all samples for an app since <cutoff>", so index on
-- (app_id, recorded_at). Also serves retention pruning by recorded_at.
CREATE INDEX idx_container_metrics_app_time
    ON container_metrics_history(app_id, recorded_at);
