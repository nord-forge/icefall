-- Phase 22: Expansion (v1.2) — final tickets

-- IF-147: Environments per project (enhanced from existing environments table)
CREATE TABLE IF NOT EXISTS project_environments (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_project_env_slug ON project_environments(project_id, slug);

CREATE TABLE IF NOT EXISTS environment_variables_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    environment_id TEXT NOT NULL REFERENCES project_environments(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value_encrypted BLOB NOT NULL,
    is_secret BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_env_vars_v2_key ON environment_variables_v2(environment_id, key);

ALTER TABLE apps ADD COLUMN project_environment_id TEXT REFERENCES project_environments(id);

-- IF-148: One-click service templates
CREATE TABLE IF NOT EXISTS service_templates (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    version TEXT,
    icon_url TEXT,
    categories TEXT,
    website TEXT,
    required_inputs TEXT NOT NULL,
    default_env TEXT,
    min_resources TEXT,
    compose_content TEXT NOT NULL,
    readme TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

ALTER TABLE apps ADD COLUMN template_id TEXT REFERENCES service_templates(id);
ALTER TABLE apps ADD COLUMN template_version TEXT;

-- IF-149: Reverse proxy management
ALTER TABLE apps ADD COLUMN has_custom_proxy_config BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE apps ADD COLUMN proxy_presets TEXT;

CREATE TABLE IF NOT EXISTS proxy_config_history (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    config TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- IF-150: Log drains
CREATE TABLE IF NOT EXISTS log_drains (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT REFERENCES apps(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    drain_type TEXT NOT NULL CHECK (drain_type IN ('loki', 'axiom', 'http')),
    config_encrypted BLOB NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_sent_at TEXT,
    error_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_log_drains_app ON log_drains(app_id);

-- IF-151: Cloudflare Tunnel
ALTER TABLE apps ADD COLUMN tunnel_enabled BOOLEAN NOT NULL DEFAULT FALSE;

-- IF-152: Automated container cleanup (global settings already added in Phase 24 per-server)
-- This adds the global cleanup schedule config
CREATE TABLE IF NOT EXISTS cleanup_schedule (
    id TEXT PRIMARY KEY NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    cron_expression TEXT NOT NULL DEFAULT '0 3 * * *',
    disk_threshold_percent INTEGER NOT NULL DEFAULT 80,
    cleanup_dangling_images BOOLEAN NOT NULL DEFAULT TRUE,
    cleanup_unused_images BOOLEAN NOT NULL DEFAULT FALSE,
    cleanup_stopped_containers BOOLEAN NOT NULL DEFAULT TRUE,
    cleanup_unused_volumes BOOLEAN NOT NULL DEFAULT FALSE,
    cleanup_unused_networks BOOLEAN NOT NULL DEFAULT TRUE,
    stopped_container_age_hours INTEGER NOT NULL DEFAULT 24,
    updated_at TEXT NOT NULL
);
