//! IF-174: data-layer tests for GitHub App integration — token storage, the
//! refresh-due query, PR comment upsert, and the app↔installation link.

#[cfg(test)]
mod github {
    use std::sync::Arc;

    use sqlx::sqlite::SqlitePoolOptions;

    use crate::db::encryption::Encryptor;
    use crate::db::models::*;
    use crate::db::sqlite::SqliteDatabase;
    use crate::db::Database;

    async fn setup_db() -> SqliteDatabase {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect to in-memory SQLite");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");
        sqlx::migrate!("src/db/migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        let encryptor = Arc::new(Encryptor::new(&Encryptor::generate_key()));
        SqliteDatabase::new_with_pool(pool, encryptor)
    }

    async fn create_installation(db: &SqliteDatabase, gh_id: i64) -> GitHubInstallation {
        db.create_github_installation(gh_id, "acme", "org")
            .await
            .expect("create installation")
    }

    #[tokio::test]
    async fn token_storage_round_trips_encrypted() {
        let db = setup_db().await;
        let inst = create_installation(&db, 555).await;

        // Fresh installation has no cached token.
        let fetched = db.get_github_installation(&inst.id).await.unwrap().unwrap();
        assert!(fetched.access_token.is_none());

        db.update_github_installation_token(555, "ghs_secrettoken", "2099-01-01T00:00:00.000Z")
            .await
            .expect("store token");

        let with_token = db.get_github_installation(&inst.id).await.unwrap().unwrap();
        assert_eq!(with_token.access_token.as_deref(), Some("ghs_secrettoken"));
        assert_eq!(
            with_token.token_expires_at.as_deref(),
            Some("2099-01-01T00:00:00.000Z")
        );
    }

    #[tokio::test]
    async fn list_installations_never_leaks_token() {
        let db = setup_db().await;
        create_installation(&db, 556).await;
        db.update_github_installation_token(556, "ghs_secret", "2099-01-01T00:00:00.000Z")
            .await
            .unwrap();

        // The bulk listing must not decrypt/expose tokens.
        let all = db.list_github_installations().await.unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].access_token.is_none());
    }

    #[tokio::test]
    async fn refresh_query_selects_expiring_and_missing_only() {
        let db = setup_db().await;
        // Needs an app linked, since the refresh query filters github_app_id NOT NULL.
        let (user, _team) = db
            .create_user_with_personal_team(&NewUser {
                email: "gh@example.com".into(),
                password_hash: "$argon2id$x".into(),
                role: "admin".into(),
            })
            .await
            .unwrap();
        let app = db
            .create_github_app(&GitHubApp {
                id: new_id(),
                name: "test-app".into(),
                app_id: 99,
                client_id: "cid".into(),
                client_secret: "secret".into(),
                private_key: "key".into(),
                webhook_secret: "whsec".into(),
                html_url: "https://github.com/apps/test".into(),
                api_url: "https://api.github.com".into(),
                owner_id: user.id.clone(),
                created_at: now_iso8601(),
                updated_at: now_iso8601(),
            })
            .await
            .unwrap();

        // Installation A: expires far in the future (should NOT be refreshed).
        create_installation(&db, 1).await;
        db.update_github_installation_app_id(1, &app.id)
            .await
            .unwrap();
        db.update_github_installation_token(1, "tok", "2099-01-01T00:00:00.000Z")
            .await
            .unwrap();

        // Installation B: no token at all (should be refreshed).
        create_installation(&db, 2).await;
        db.update_github_installation_app_id(2, &app.id)
            .await
            .unwrap();

        // Installation C: token already expired (should be refreshed).
        create_installation(&db, 3).await;
        db.update_github_installation_app_id(3, &app.id)
            .await
            .unwrap();
        db.update_github_installation_token(3, "old", "2000-01-01T00:00:00.000Z")
            .await
            .unwrap();

        let threshold = "2026-01-01T00:00:00.000Z";
        let due = db
            .list_installations_needing_token_refresh(threshold)
            .await
            .unwrap();
        let due_ids: Vec<i64> = due.iter().map(|i| i.installation_id).collect();
        assert!(due_ids.contains(&2), "missing-token install should be due");
        assert!(due_ids.contains(&3), "expired install should be due");
        assert!(!due_ids.contains(&1), "future install should not be due");
    }

    #[tokio::test]
    async fn pr_comment_upsert_tracks_then_updates() {
        let db = setup_db().await;
        let (_u, team) = db
            .create_user_with_personal_team(&NewUser {
                email: "pr@example.com".into(),
                password_hash: "$argon2id$x".into(),
                role: "admin".into(),
            })
            .await
            .unwrap();
        let app = db
            .create_app(&NewApp {
                name: "pr-app".into(),
                team_id: team.id,
                git_repo: Some("https://github.com/acme/pr-app".into()),
                git_branch: "main".into(),
                framework: None,
                image_ref: None,
                compose_content: None,
                deploy_mode: None,
                server_id: None,
            })
            .await
            .unwrap();

        assert!(db
            .get_github_pr_comment(&app.id, 42)
            .await
            .unwrap()
            .is_none());

        db.upsert_github_pr_comment(&app.id, 7, "acme/pr-app", 42, 1001)
            .await
            .unwrap();
        let first = db
            .get_github_pr_comment(&app.id, 42)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.comment_id, 1001);

        // Upsert again for the same (app, PR) updates the comment id in place.
        db.upsert_github_pr_comment(&app.id, 7, "acme/pr-app", 42, 2002)
            .await
            .unwrap();
        let updated = db
            .get_github_pr_comment(&app.id, 42)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.comment_id, 2002);
        assert_eq!(updated.id, first.id, "same tracking row reused");
    }

    #[tokio::test]
    async fn app_installation_link_persists() {
        let db = setup_db().await;
        let (_u, team) = db
            .create_user_with_personal_team(&NewUser {
                email: "link@example.com".into(),
                password_hash: "$argon2id$x".into(),
                role: "admin".into(),
            })
            .await
            .unwrap();
        let inst = create_installation(&db, 909).await;
        let app = db
            .create_app(&NewApp {
                name: "link-app".into(),
                team_id: team.id,
                git_repo: Some("https://github.com/acme/link".into()),
                git_branch: "main".into(),
                framework: None,
                image_ref: None,
                compose_content: None,
                deploy_mode: None,
                server_id: None,
            })
            .await
            .unwrap();
        assert!(app.github_installation_id.is_none());

        let updated = db
            .update_app(
                &app.id,
                &UpdateApp {
                    github_installation_id: Some(Some(inst.id.clone())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(
            updated.github_installation_id.as_deref(),
            Some(inst.id.as_str())
        );
    }
}
