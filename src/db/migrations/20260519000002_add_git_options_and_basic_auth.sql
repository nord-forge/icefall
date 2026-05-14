-- Phase 24: IF-224 (git submodule/LFS), IF-212 (HTTP basic auth), IF-211 (pre-deploy commands)
-- IF-216 (disk alerts) adds fields to servers table

-- IF-224: Git clone options per app
ALTER TABLE apps ADD COLUMN git_submodules_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE apps ADD COLUMN git_lfs_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE apps ADD COLUMN git_shallow_clone BOOLEAN NOT NULL DEFAULT TRUE;

-- IF-212: HTTP basic auth per app
ALTER TABLE apps ADD COLUMN basic_auth_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE apps ADD COLUMN basic_auth_username TEXT;
ALTER TABLE apps ADD COLUMN basic_auth_password_hash TEXT;

-- IF-211: Pre-deploy commands
ALTER TABLE apps ADD COLUMN pre_deploy_commands TEXT;

-- IF-216: Server disk usage alert thresholds
ALTER TABLE servers ADD COLUMN disk_alert_enabled BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE servers ADD COLUMN disk_alert_warning_threshold INTEGER NOT NULL DEFAULT 80;
ALTER TABLE servers ADD COLUMN disk_alert_critical_threshold INTEGER NOT NULL DEFAULT 90;
ALTER TABLE servers ADD COLUMN disk_alert_state TEXT NOT NULL DEFAULT 'normal';
