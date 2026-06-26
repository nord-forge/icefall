-- Track when each OAuth provider first became fully configured + enabled, for
-- the settings UI's concise "Connected since {date}" status. Nullable: a
-- provider that has never been configured stays NULL. Set on the transition to
-- (client_id + secret present AND enabled); cleared when a provider is disabled.

ALTER TABLE oauth_settings ADD COLUMN github_configured_at TEXT;
ALTER TABLE oauth_settings ADD COLUMN google_configured_at TEXT;
