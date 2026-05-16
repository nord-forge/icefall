-- GitHub App integration (full app credentials, not just installations)
CREATE TABLE IF NOT EXISTS github_apps (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    app_id INTEGER NOT NULL,
    client_id TEXT NOT NULL,
    client_secret_encrypted BLOB NOT NULL,
    private_key_encrypted BLOB NOT NULL,
    webhook_secret_encrypted BLOB NOT NULL,
    html_url TEXT NOT NULL DEFAULT 'https://github.com',
    api_url TEXT NOT NULL DEFAULT 'https://api.github.com',
    owner_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Add github_app_id to github_installations to link installations to apps
ALTER TABLE github_installations ADD COLUMN github_app_id TEXT REFERENCES github_apps(id);
