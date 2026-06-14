//! IF-179: data-layer tests for scheduled deploys — the `scheduled`/`missed`
//! status states, the due-query the scheduler polls, and the claim/reschedule
//! transitions. Docker/Caddy-independent.

#[cfg(test)]
mod scheduled_deploys {
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

    /// Create an app with one production environment and return (app, env_id).
    async fn create_app_with_env(db: &SqliteDatabase) -> (App, String) {
        let (_user, team) = db
            .create_user_with_personal_team(&NewUser {
                email: "sched@example.com".to_string(),
                password_hash: "$argon2id$test".to_string(),
                role: "admin".to_string(),
            })
            .await
            .expect("create user with personal team");

        let app = db
            .create_app(&NewApp {
                name: "scheduler-app".to_string(),
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
            .expect("create app");

        let env = db
            .create_environment(&NewEnvironment {
                app_id: app.id.clone(),
                name: "production".to_string(),
                env_type: "production".to_string(),
                branch: Some("main".to_string()),
            })
            .await
            .expect("create environment");

        (app, env.id)
    }

    fn new_scheduled(app_id: &str, env_id: &str, scheduled_at: Option<String>) -> NewDeploy {
        NewDeploy {
            app_id: app_id.to_string(),
            environment_id: env_id.to_string(),
            git_sha: None,
            server_id: None,
            tag: None,
            no_cache: false,
            scheduled_at,
        }
    }

    #[tokio::test]
    async fn scheduled_deploy_is_parked_not_pending() {
        let db = setup_db().await;
        let (app, env_id) = create_app_with_env(&db).await;

        let future = (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
        let deploy = db
            .create_deploy(&new_scheduled(&app.id, &env_id, Some(future.clone())))
            .await
            .expect("create scheduled deploy");

        assert_eq!(deploy.status, "scheduled");
        assert_eq!(deploy.scheduled_at.as_deref(), Some(future.as_str()));
        // A scheduled deploy has not started yet.
        assert!(deploy.started_at.is_none());

        // Not due yet, so the scheduler must not pick it up.
        let due = db.list_due_scheduled_deploys().await.expect("list due");
        assert!(due.is_empty(), "future deploy should not be due");
    }

    #[tokio::test]
    async fn immediate_deploy_is_pending() {
        let db = setup_db().await;
        let (app, env_id) = create_app_with_env(&db).await;

        let deploy = db
            .create_deploy(&new_scheduled(&app.id, &env_id, None))
            .await
            .expect("create immediate deploy");

        assert_eq!(deploy.status, "pending");
        assert!(deploy.scheduled_at.is_none());
        assert!(deploy.started_at.is_some());
    }

    #[tokio::test]
    async fn due_deploy_can_be_claimed_once() {
        let db = setup_db().await;
        let (app, env_id) = create_app_with_env(&db).await;

        let past = (chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339();
        let deploy = db
            .create_deploy(&new_scheduled(&app.id, &env_id, Some(past)))
            .await
            .expect("create due deploy");

        let due = db.list_due_scheduled_deploys().await.expect("list due");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, deploy.id);

        // First claim wins; the second sees it already gone.
        assert!(db.start_scheduled_deploy(&deploy.id).await.unwrap());
        assert!(!db.start_scheduled_deploy(&deploy.id).await.unwrap());

        let after = db.get_deploy(&deploy.id).await.unwrap().unwrap();
        assert_eq!(after.status, "pending");
        assert!(after.started_at.is_some());

        // No longer scheduled, so it drops out of the due query.
        let due = db.list_due_scheduled_deploys().await.expect("list due");
        assert!(due.is_empty());
    }

    #[tokio::test]
    async fn reschedule_only_works_while_scheduled() {
        let db = setup_db().await;
        let (app, env_id) = create_app_with_env(&db).await;

        let soon = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();
        let deploy = db
            .create_deploy(&new_scheduled(&app.id, &env_id, Some(soon)))
            .await
            .expect("create scheduled deploy");

        let later = (chrono::Utc::now() + chrono::Duration::hours(6)).to_rfc3339();
        assert!(db.reschedule_deploy(&deploy.id, &later).await.unwrap());
        let updated = db.get_deploy(&deploy.id).await.unwrap().unwrap();
        assert_eq!(updated.scheduled_at.as_deref(), Some(later.as_str()));

        // Once it has been claimed it is no longer reschedulable.
        db.start_scheduled_deploy(&deploy.id).await.unwrap();
        assert!(!db.reschedule_deploy(&deploy.id, &later).await.unwrap());
    }

    #[tokio::test]
    async fn missed_status_is_persisted() {
        let db = setup_db().await;
        let (app, env_id) = create_app_with_env(&db).await;

        let past = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        let deploy = db
            .create_deploy(&new_scheduled(&app.id, &env_id, Some(past)))
            .await
            .expect("create stale deploy");

        // The 'missed' state must satisfy the deploys.status CHECK constraint
        // (the whole point of the IF-179 migration).
        db.update_deploy_status(&deploy.id, "missed", Some("offline"))
            .await
            .expect("mark missed");

        let after = db.get_deploy(&deploy.id).await.unwrap().unwrap();
        assert_eq!(after.status, "missed");
    }
}
