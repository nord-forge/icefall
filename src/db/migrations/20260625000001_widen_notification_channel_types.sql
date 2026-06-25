-- Widen the notification channel CHECK constraint.
--
-- The API (src/api/routes/notifications/channels.rs) and dispatcher
-- (src/api/routes/notifications/dispatch.rs) already accept and handle 'ntfy',
-- 'slack' and 'discord', but the baseline CHECK constraint only permitted
-- 'smtp', 'webhook' and 'plunk'. Creating any of the newer channels therefore
-- failed at INSERT with "CHECK constraint failed", surfacing as a bare 500.
--
-- SQLite cannot ALTER a CHECK constraint in place, so `notifications` is rebuilt.
-- Important: sqlx keeps PRAGMA foreign_keys ON for the whole connection, and
-- with FK enforcement on, `ALTER TABLE ... RENAME` ALWAYS rewrites child
-- foreign-key references to the new name (the legacy_alter_table pragma is
-- ignored while FK is on, and foreign_keys can't be toggled inside the
-- transaction sqlx wraps each migration in). That would leave
-- notification_rules pointing at the dropped "notifications_old" table and
-- break later cascades (e.g. deleting an app). So we rebuild BOTH tables in
-- dependency order: stash and drop the child first (so nothing references the
-- parent mid-rebuild), rebuild the parent, then recreate the child fresh with
-- its FK authored against "notifications".

-- 1. Stash child rows, then drop the child so it no longer references the parent.
CREATE TEMP TABLE _notification_rules_backup AS SELECT * FROM notification_rules;
DROP TABLE notification_rules;

-- 2. Rebuild the parent with the widened CHECK constraint.
ALTER TABLE notifications RENAME TO notifications_old;

CREATE TABLE notifications (
    id TEXT PRIMARY KEY NOT NULL,
    channel_type TEXT NOT NULL CHECK (channel_type IN ('smtp', 'webhook', 'plunk', 'ntfy', 'slack', 'discord')),
    config_encrypted BLOB NOT NULL,
    team_id TEXT REFERENCES teams(id),
    created_at TEXT NOT NULL
);

INSERT INTO notifications (id, channel_type, config_encrypted, team_id, created_at)
SELECT id, channel_type, config_encrypted, team_id, created_at
FROM notifications_old;

DROP TABLE notifications_old;

-- 3. Recreate the child exactly as in the baseline; its FK now points at the
--    rebuilt "notifications". Restore rows and the index.
CREATE TABLE notification_rules (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    notification_id TEXT NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('deploy_success', 'deploy_failure', 'health_down', 'health_recovered', 'auto_restart'))
);

INSERT INTO notification_rules (id, app_id, notification_id, event_type)
SELECT id, app_id, notification_id, event_type
FROM _notification_rules_backup;

DROP TABLE _notification_rules_backup;

CREATE INDEX idx_notification_rules_app_id ON notification_rules(app_id);
