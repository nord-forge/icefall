//! Regression tests for backend bugs that caused server-side 500s:
//! - log_drains queried a non-existent `config` column (schema has
//!   `config_encrypted`) and never encrypted the config
//! - clone_app omitted the NOT NULL `team_id` column
//! - the update_state singleton row was never seeded

#[cfg(test)]
mod server_bug_regression {
    use std::sync::Arc;

    use sqlx::sqlite::SqlitePoolOptions;

    use crate::db::encryption::Encryptor;
    use crate::db::models::*;
    use crate::db::prelude::*;
    use crate::db::sqlite::SqliteDatabase;

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

    async fn seed_team(db: &SqliteDatabase) -> Team {
        let owner = db
            .create_user(&NewUser {
                email: "owner@example.com".to_string(),
                password_hash: "$argon2id$test".to_string(),
                role: "admin".to_string(),
            })
            .await
            .expect("create user");
        db.create_team(&NewTeam {
            name: "Team".to_string(),
            slug: "team".to_string(),
            owner_id: owner.id,
        })
        .await
        .expect("create team")
    }

    // log_drains: create/list/get/update must hit config_encrypted and
    // round-trip the plaintext config through encryption (no 500).
    #[tokio::test]
    async fn log_drain_round_trips_encrypted_config() {
        let db = setup_db().await;

        let created = db
            .create_log_drain(&NewLogDrain {
                app_id: None,
                name: "loki".to_string(),
                drain_type: "loki".to_string(),
                config: r#"{"url":"https://loki","token":"secret"}"#.to_string(),
            })
            .await
            .expect("create log drain");
        assert_eq!(created.config, r#"{"url":"https://loki","token":"secret"}"#);

        let fetched = db
            .get_log_drain(&created.id)
            .await
            .expect("get log drain")
            .expect("log drain exists");
        assert_eq!(fetched.config, created.config);

        let global = db.list_global_log_drains().await.expect("list global");
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].config, created.config);

        let updated = db
            .update_log_drain(
                &created.id,
                &NewLogDrain {
                    app_id: None,
                    name: "loki-2".to_string(),
                    drain_type: "loki".to_string(),
                    config: r#"{"url":"https://loki2"}"#.to_string(),
                },
            )
            .await
            .expect("update log drain");
        assert_eq!(updated.name, "loki-2");
        assert_eq!(updated.config, r#"{"url":"https://loki2"}"#);
    }

    // The encrypted config must not be stored as plaintext on disk.
    #[tokio::test]
    async fn log_drain_config_is_encrypted_at_rest() {
        let db = setup_db().await;
        let drain = db
            .create_log_drain(&NewLogDrain {
                app_id: None,
                name: "axiom".to_string(),
                drain_type: "axiom".to_string(),
                config: "PLAINTEXT_TOKEN_MARKER".to_string(),
            })
            .await
            .expect("create");

        let raw: Vec<u8> =
            sqlx::query_scalar("SELECT config_encrypted FROM log_drains WHERE id = ?")
                .bind(&drain.id)
                .fetch_one(db.pool_for_test())
                .await
                .expect("fetch raw");
        let as_str = String::from_utf8_lossy(&raw);
        assert!(
            !as_str.contains("PLAINTEXT_TOKEN_MARKER"),
            "config stored in plaintext"
        );
    }

    // clone_app must carry team_id (NOT NULL) so the insert succeeds.
    #[tokio::test]
    async fn clone_app_preserves_team_id() {
        let db = setup_db().await;
        let team = seed_team(&db).await;

        let source = db
            .create_app(&NewApp {
                name: "source".to_string(),
                team_id: team.id.clone(),
                git_repo: None,
                git_branch: "main".to_string(),
                framework: None,
                image_ref: None,
                compose_content: None,
                deploy_mode: None,
                server_id: None,
            })
            .await
            .expect("create source app");

        let clone = db
            .clone_app(&source.id, "source-copy", None, None)
            .await
            .expect("clone app");
        assert_eq!(clone.team_id, team.id);
        assert_eq!(clone.name, "source-copy");
    }

    // update_state singleton row is seeded by migration -> no "no rows" 500.
    #[tokio::test]
    async fn update_state_is_seeded() {
        let db = setup_db().await;
        let state = db
            .get_update_state()
            .await
            .expect("update_state row exists");
        assert_eq!(state.channel, "stable");
    }
}
