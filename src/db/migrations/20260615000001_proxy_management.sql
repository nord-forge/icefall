-- IF-149: Reverse proxy management UI
--
-- Adds storage for raw advanced-mode proxy config (custom_proxy_config) and a
-- singleton table of global proxy defaults. proxy_presets, proxy_config_history
-- and has_custom_proxy_config already exist in the baseline schema.

-- Raw Caddy JSON saved when an app enters advanced mode. NULL while the app
-- relies on presets + auto-generation (has_custom_proxy_config = false).
ALTER TABLE apps ADD COLUMN custom_proxy_config TEXT;

-- Global proxy defaults (single row, id = 'global'). Applied on top of every
-- auto-generated app route.
CREATE TABLE proxy_settings (
    id TEXT PRIMARY KEY NOT NULL DEFAULT 'global',
    default_headers TEXT,           -- JSON object of header name -> value
    default_rate_limit TEXT,        -- JSON: { "enabled": bool, "requests": n, "window": "minute"|"second", "burst": n }
    force_https BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TEXT NOT NULL
);

INSERT INTO proxy_settings (id, force_https, updated_at)
VALUES ('global', TRUE, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
