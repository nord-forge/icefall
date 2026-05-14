-- Phase 25 batch 2: M-sized parity gap tickets

-- IF-163: Post-deploy commands (field already added in Phase 24 migration as pre_deploy_commands)
-- Adding the post_deploy counterpart
ALTER TABLE apps ADD COLUMN post_deploy_commands TEXT;

-- IF-167: Notification alert wiring — no schema changes needed (uses existing notification system)

-- IF-168: Granular API token ability scoping
ALTER TABLE api_tokens ADD COLUMN abilities TEXT;

-- IF-169: SSH key management
CREATE TABLE IF NOT EXISTS ssh_keys (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    public_key TEXT NOT NULL,
    private_key_encrypted BLOB NOT NULL,
    fingerprint TEXT NOT NULL,
    key_type TEXT NOT NULL DEFAULT 'ed25519' CHECK (key_type IN ('ed25519', 'rsa')),
    last_used_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ssh_keys_user_id ON ssh_keys(user_id);

-- App-level SSH key selection for private repo cloning
ALTER TABLE apps ADD COLUMN ssh_key_id TEXT REFERENCES ssh_keys(id);

-- IF-170: Container registry credentials
CREATE TABLE IF NOT EXISTS registries (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    username_encrypted BLOB NOT NULL,
    password_encrypted BLOB NOT NULL,
    registry_type TEXT NOT NULL DEFAULT 'custom' CHECK (registry_type IN ('dockerhub', 'ghcr', 'gitlab', 'custom')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- IF-172: Public port / TCP proxy
CREATE TABLE IF NOT EXISTS public_ports (
    id TEXT PRIMARY KEY NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    port INTEGER NOT NULL UNIQUE,
    ip_whitelist TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_public_ports_resource ON public_ports(resource_type, resource_id);

-- IF-174: GitHub App integration
CREATE TABLE IF NOT EXISTS github_installations (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id INTEGER NOT NULL UNIQUE,
    account_login TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('user', 'org')),
    access_token_encrypted BLOB,
    token_expires_at TEXT,
    created_at TEXT NOT NULL
);
