//! IF-191: data-layer tests for per-container metrics persistence — recording,
//! avg/peak aggregation over a window, and retention pruning.

#[cfg(test)]
mod container_metrics {
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
            .expect("connect");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("fk");
        sqlx::migrate!("src/db/migrations")
            .run(&pool)
            .await
            .expect("migrate");
        let encryptor = Arc::new(Encryptor::new(&Encryptor::generate_key()));
        SqliteDatabase::new_with_pool(pool, encryptor)
    }

    async fn make_app(db: &SqliteDatabase) -> String {
        let (_u, team) = db
            .create_user_with_personal_team(&NewUser {
                email: "m@example.com".into(),
                password_hash: "$argon2id$test".into(),
                role: "admin".into(),
            })
            .await
            .expect("user+team");
        db.create_app(&NewApp {
            name: "metrics-app".into(),
            team_id: team.id,
            git_repo: None,
            git_branch: "main".into(),
            framework: None,
            image_ref: None,
            compose_content: None,
            deploy_mode: None,
            server_id: None,
        })
        .await
        .expect("app")
        .id
    }

    #[tokio::test]
    async fn aggregates_avg_and_peak() {
        let db = setup_db().await;
        let app_id = make_app(&db).await;

        for (cpu, mem) in [(10.0, 100), (30.0, 300), (20.0, 200)] {
            db.record_container_metrics(&NewContainerMetricsRecord {
                app_id: app_id.clone(),
                cpu_percent: cpu,
                memory_usage_bytes: mem,
                memory_limit_bytes: 1000,
            })
            .await
            .expect("record");
        }

        let stats = db.container_usage_stats(7).await.expect("stats");
        assert_eq!(stats.len(), 1);
        let s = &stats[0];
        assert_eq!(s.app_id, app_id);
        assert_eq!(s.sample_count, 3);
        assert!((s.avg_cpu_percent - 20.0).abs() < 0.001);
        assert!((s.peak_cpu_percent - 30.0).abs() < 0.001);
        assert_eq!(s.peak_memory_bytes, 300);
        assert_eq!(s.avg_memory_bytes, 200);
        assert_eq!(s.memory_limit_bytes, 1000);
    }

    #[tokio::test]
    async fn prune_removes_old_rows() {
        let db = setup_db().await;
        let app_id = make_app(&db).await;
        db.record_container_metrics(&NewContainerMetricsRecord {
            app_id,
            cpu_percent: 5.0,
            memory_usage_bytes: 50,
            memory_limit_bytes: 500,
        })
        .await
        .expect("record");

        // Nothing older than a huge window — keeps the row.
        let removed = db.prune_container_metrics(3650).await.expect("prune");
        assert_eq!(removed, 0);
        assert_eq!(db.container_usage_stats(7).await.unwrap().len(), 1);

        // keep_days = 0 ⇒ cutoff is "now", so the just-written row is older. It
        // should be pruned (the row's recorded_at is strictly before now()).
        let removed = db.prune_container_metrics(0).await.expect("prune");
        assert_eq!(removed, 1);
        assert!(db.container_usage_stats(7).await.unwrap().is_empty());
    }
}
