//! IF-149: data-layer tests for reverse proxy management — preset persistence,
//! advanced-mode custom config flag transitions, config-history pruning to the
//! last 10 snapshots, and global proxy settings upsert. Docker/Caddy-independent.

#[cfg(test)]
mod proxy {
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

    async fn create_app(db: &SqliteDatabase) -> App {
        let (_user, team) = db
            .create_user_with_personal_team(&NewUser {
                email: "proxy@example.com".to_string(),
                password_hash: "$argon2id$test".to_string(),
                role: "admin".to_string(),
            })
            .await
            .expect("create user with personal team");

        db.create_app(&NewApp {
            name: "proxy-app".to_string(),
            team_id: team.id,
            git_repo: Some("https://github.com/test/repo".to_string()),
            git_branch: "main".to_string(),
            framework: None,
            image_ref: None,
            compose_content: None,
            deploy_mode: None,
            server_id: None,
        })
        .await
        .expect("create app")
    }

    #[tokio::test]
    async fn new_app_has_no_custom_proxy_config() {
        let db = setup_db().await;
        let app = create_app(&db).await;
        assert!(!app.has_custom_proxy_config);
        assert!(app.custom_proxy_config.is_none());
        assert!(app.proxy_presets.is_none());
    }

    #[tokio::test]
    async fn set_presets_persists_without_enabling_advanced_mode() {
        let db = setup_db().await;
        let app = create_app(&db).await;

        let presets = r#"{"force_https":true,"redirects":[],"headers":[]}"#;
        db.set_proxy_presets(&app.id, presets)
            .await
            .expect("set presets");

        let reloaded = db.get_app(&app.id).await.unwrap().unwrap();
        assert_eq!(reloaded.proxy_presets.as_deref(), Some(presets));
        // Presets must NOT flip the app into advanced mode.
        assert!(!reloaded.has_custom_proxy_config);
    }

    #[tokio::test]
    async fn set_custom_config_enables_advanced_mode_and_clear_reverts() {
        let db = setup_db().await;
        let app = create_app(&db).await;

        db.set_custom_proxy_config(&app.id, r#"{"apps":{}}"#)
            .await
            .expect("set custom config");

        let after_set = db.get_app(&app.id).await.unwrap().unwrap();
        assert!(after_set.has_custom_proxy_config);
        assert_eq!(
            after_set.custom_proxy_config.as_deref(),
            Some(r#"{"apps":{}}"#)
        );

        db.clear_custom_proxy_config(&app.id)
            .await
            .expect("clear custom config");

        let after_clear = db.get_app(&app.id).await.unwrap().unwrap();
        assert!(!after_clear.has_custom_proxy_config);
        assert!(after_clear.custom_proxy_config.is_none());
    }

    #[tokio::test]
    async fn config_history_prunes_to_last_ten() {
        let db = setup_db().await;
        let app = create_app(&db).await;

        for i in 0..15 {
            db.record_proxy_config_history(&app.id, &format!("config-{i}"))
                .await
                .expect("record history");
        }

        let history = db
            .list_proxy_config_history(&app.id)
            .await
            .expect("list history");
        assert_eq!(history.len(), 10, "history should be capped at 10");
        // Newest first: the most recent snapshot is config-14.
        assert_eq!(history[0].config, "config-14");
        // The oldest retained is config-5 (0..4 pruned).
        assert_eq!(history[9].config, "config-5");
    }

    #[tokio::test]
    async fn latest_history_returns_most_recent() {
        let db = setup_db().await;
        let app = create_app(&db).await;

        assert!(db
            .latest_proxy_config_history(&app.id)
            .await
            .unwrap()
            .is_none());

        db.record_proxy_config_history(&app.id, "first")
            .await
            .unwrap();
        db.record_proxy_config_history(&app.id, "second")
            .await
            .unwrap();

        let latest = db
            .latest_proxy_config_history(&app.id)
            .await
            .unwrap()
            .expect("a snapshot");
        assert_eq!(latest.config, "second");
    }

    #[tokio::test]
    async fn history_is_scoped_per_app() {
        let db = setup_db().await;
        let app_a = create_app(&db).await;
        // Second app under a different team/user.
        let (_u, team_b) = db
            .create_user_with_personal_team(&NewUser {
                email: "proxy-b@example.com".to_string(),
                password_hash: "$argon2id$test".to_string(),
                role: "admin".to_string(),
            })
            .await
            .unwrap();
        let app_b = db
            .create_app(&NewApp {
                name: "proxy-app-b".to_string(),
                team_id: team_b.id,
                git_repo: None,
                git_branch: "main".to_string(),
                framework: None,
                image_ref: None,
                compose_content: None,
                deploy_mode: None,
                server_id: None,
            })
            .await
            .unwrap();

        db.record_proxy_config_history(&app_a.id, "a-config")
            .await
            .unwrap();
        db.record_proxy_config_history(&app_b.id, "b-config")
            .await
            .unwrap();

        let a_hist = db.list_proxy_config_history(&app_a.id).await.unwrap();
        let b_hist = db.list_proxy_config_history(&app_b.id).await.unwrap();
        assert_eq!(a_hist.len(), 1);
        assert_eq!(b_hist.len(), 1);
        assert_eq!(a_hist[0].config, "a-config");
        assert_eq!(b_hist[0].config, "b-config");
    }

    #[tokio::test]
    async fn global_proxy_settings_seeded_and_updatable() {
        let db = setup_db().await;

        // Migration seeds a default global row.
        let initial = db.get_proxy_settings().await.expect("get settings");
        assert_eq!(initial.id, "global");
        assert!(initial.force_https);
        assert!(initial.default_headers.is_none());

        let update = UpdateProxySettings {
            default_headers: Some(Some(r#"{"X-Frame-Options":"DENY"}"#.to_string())),
            default_rate_limit: None,
            force_https: Some(false),
        };
        let updated = db.update_proxy_settings(&update).await.expect("update");
        assert!(!updated.force_https);
        assert_eq!(
            updated.default_headers.as_deref(),
            Some(r#"{"X-Frame-Options":"DENY"}"#)
        );

        // Unspecified field (rate limit) stays unchanged across the update.
        assert!(updated.default_rate_limit.is_none());
    }

    #[tokio::test]
    async fn set_presets_on_missing_app_errors() {
        let db = setup_db().await;
        let err = db.set_proxy_presets("nonexistent", "{}").await;
        assert!(err.is_err());
    }
}
