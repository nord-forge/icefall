-- IF-174: GitHub App integration
--
-- Links apps to the GitHub installation that manages them (for status checks,
-- PR comments, and webhook automation), and tracks the PR comments Icefall
-- posts so they can be edited rather than duplicated.
--
-- github_installations.access_token_encrypted + token_expires_at already exist
-- in the baseline schema.

-- Which GitHub installation deploys this app — selects the App/private key used
-- to authenticate status checks, comments, and webhook creation. NULL for apps
-- connected by manual webhook (the pre-IF-174 path).
ALTER TABLE apps ADD COLUMN github_installation_id TEXT REFERENCES github_installations(id);

-- One tracked PR comment per (app, PR). Lets Icefall edit its preview-env
-- comment on subsequent pushes instead of posting a new one each time.
CREATE TABLE github_pr_comments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    installation_id INTEGER NOT NULL,
    repo_full_name TEXT NOT NULL,
    pr_number INTEGER NOT NULL,
    comment_id INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (app_id, pr_number)
);
