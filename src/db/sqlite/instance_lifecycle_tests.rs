//! QA-264: integration tests for the multi-instance / load-balancing data
//! lifecycle.
//!
//! These exercise the parts of Phase 31 that do not require a live Docker or
//! Caddy: the `app_instances` data model, scale-up / scale-down state
//! transitions, per-server instance queries, and the Caddy multi-upstream
//! config generation. Docker- and Caddy-network-dependent scenarios (real
//! zero-downtime rolling deploys, live traffic distribution) are out of scope
//! for unit-level tests and are covered by manual / staging verification.

#[cfg(test)]
mod instance_lifecycle {
    use std::sync::Arc;

    use sqlx::sqlite::SqlitePoolOptions;

    use crate::caddy::types::{caddy_lb_policy, CaddyRoute};
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

    async fn create_test_app(db: &SqliteDatabase, name: &str) -> App {
        db.create_app(&NewApp {
            name: name.to_string(),
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

    async fn create_worker(db: &SqliteDatabase, name: &str) -> Server {
        db.create_server(&NewServer {
            name: name.to_string(),
            host: format!("{name}.example.com"),
            role: "worker".to_string(),
            token_hash: None,
            labels: None,
            resources: None,
            public_key: None,
        })
        .await
        .expect("create server")
    }

    /// An instance running on a server, with a container and host port.
    async fn add_running_instance(
        db: &SqliteDatabase,
        app_id: &str,
        server_id: &str,
        port: i64,
    ) -> AppInstance {
        db.create_app_instance(&NewAppInstance {
            app_id: app_id.to_string(),
            server_id: server_id.to_string(),
            status: "running".to_string(),
            container_id: Some(format!("container-{port}")),
            host_port: Some(port),
        })
        .await
        .expect("create instance")
    }

    // --- app model: desired_instances + lb config ---

    #[tokio::test]
    async fn new_app_defaults_to_single_instance() {
        let db = setup_db().await;
        let app = create_test_app(&db, "single").await;
        assert_eq!(app.desired_instances, 1);
        assert_eq!(app.lb_policy, "round_robin");
        assert_eq!(app.lb_health_check_path, "/");
        assert!(!app.lb_sticky_sessions);
    }

    #[tokio::test]
    async fn scaling_updates_desired_instance_count() {
        let db = setup_db().await;
        let app = create_test_app(&db, "scaler").await;

        let updated = db
            .update_app(
                &app.id,
                &UpdateApp {
                    desired_instances: Some(3),
                    ..Default::default()
                },
            )
            .await
            .expect("update app");
        assert_eq!(updated.desired_instances, 3);
    }

    #[tokio::test]
    async fn lb_config_persists_policy_health_path_and_sticky() {
        let db = setup_db().await;
        let app = create_test_app(&db, "lb").await;

        let updated = db
            .update_app(
                &app.id,
                &UpdateApp {
                    lb_policy: Some("least_conn".to_string()),
                    lb_health_check_path: Some("/healthz".to_string()),
                    lb_sticky_sessions: Some(true),
                    ..Default::default()
                },
            )
            .await
            .expect("update lb config");
        assert_eq!(updated.lb_policy, "least_conn");
        assert_eq!(updated.lb_health_check_path, "/healthz");
        assert!(updated.lb_sticky_sessions);
    }

    // --- app_instances CRUD ---

    #[tokio::test]
    async fn instances_are_listed_for_their_app() {
        let db = setup_db().await;
        let app = create_test_app(&db, "listed").await;
        let worker = create_worker(&db, "w1").await;

        add_running_instance(&db, &app.id, &worker.id, 8001).await;
        add_running_instance(&db, &app.id, &worker.id, 8002).await;

        let instances = db.list_app_instances(&app.id).await.expect("list");
        assert_eq!(instances.len(), 2);
        assert!(instances.iter().all(|i| i.app_id == app.id));
    }

    #[tokio::test]
    async fn instance_status_updates_independently() {
        let db = setup_db().await;
        let app = create_test_app(&db, "status").await;
        let worker = create_worker(&db, "w1").await;

        let a = add_running_instance(&db, &app.id, &worker.id, 8001).await;
        let b = add_running_instance(&db, &app.id, &worker.id, 8002).await;

        // Mark only instance `a` failed.
        db.update_app_instance(
            &a.id,
            &UpdateAppInstance {
                status: Some("failed".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("update a");

        let instances = db.list_app_instances(&app.id).await.expect("list");
        let a_now = instances.iter().find(|i| i.id == a.id).unwrap();
        let b_now = instances.iter().find(|i| i.id == b.id).unwrap();
        assert_eq!(a_now.status, "failed");
        assert_eq!(b_now.status, "running");
    }

    // --- scale up / scale down (state reconciliation) ---

    #[tokio::test]
    async fn scale_up_adds_instances_to_reach_desired() {
        let db = setup_db().await;
        let app = create_test_app(&db, "up").await;
        let worker = create_worker(&db, "w1").await;

        // Start at 1 instance.
        add_running_instance(&db, &app.id, &worker.id, 8001).await;
        db.update_app(
            &app.id,
            &UpdateApp {
                desired_instances: Some(3),
                ..Default::default()
            },
        )
        .await
        .expect("set desired 3");

        // Reconcile: add the shortfall.
        let current = db.list_app_instances(&app.id).await.unwrap().len() as i64;
        let target = db
            .get_app(&app.id)
            .await
            .unwrap()
            .unwrap()
            .desired_instances;
        for port in 0..(target - current) {
            add_running_instance(&db, &app.id, &worker.id, 8100 + port).await;
        }

        assert_eq!(db.list_app_instances(&app.id).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn scale_down_removes_excess_instances() {
        let db = setup_db().await;
        let app = create_test_app(&db, "down").await;
        let worker = create_worker(&db, "w1").await;

        let mut instances = Vec::new();
        for port in 0..3 {
            instances.push(add_running_instance(&db, &app.id, &worker.id, 8001 + port).await);
        }
        db.update_app(
            &app.id,
            &UpdateApp {
                desired_instances: Some(1),
                ..Default::default()
            },
        )
        .await
        .expect("set desired 1");

        // Reconcile: tear down the excess.
        let target = db
            .get_app(&app.id)
            .await
            .unwrap()
            .unwrap()
            .desired_instances as usize;
        for excess in instances.iter().skip(target) {
            db.delete_app_instance(&excess.id).await.expect("delete");
        }

        assert_eq!(db.list_app_instances(&app.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn scale_to_zero_removes_all_instances() {
        let db = setup_db().await;
        let app = create_test_app(&db, "zero").await;
        let worker = create_worker(&db, "w1").await;

        for port in 0..2 {
            add_running_instance(&db, &app.id, &worker.id, 8001 + port).await;
        }
        for inst in db.list_app_instances(&app.id).await.unwrap() {
            db.delete_app_instance(&inst.id).await.expect("delete");
        }
        assert!(db.list_app_instances(&app.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn deleting_app_cascades_to_its_instances() {
        let db = setup_db().await;
        let app = create_test_app(&db, "cascade").await;
        let worker = create_worker(&db, "w1").await;
        add_running_instance(&db, &app.id, &worker.id, 8001).await;

        db.delete_app(&app.id).await.expect("delete app");
        assert!(db.list_app_instances(&app.id).await.unwrap().is_empty());
    }

    // --- per-server instance queries (FE-263) ---

    #[tokio::test]
    async fn instances_can_be_queried_by_server() {
        let db = setup_db().await;
        let app_a = create_test_app(&db, "app-a").await;
        let app_b = create_test_app(&db, "app-b").await;
        let w1 = create_worker(&db, "w1").await;
        let w2 = create_worker(&db, "w2").await;

        // w1: 2 instances of app-a + 1 of app-b. w2: 1 of app-b.
        add_running_instance(&db, &app_a.id, &w1.id, 8001).await;
        add_running_instance(&db, &app_a.id, &w1.id, 8002).await;
        add_running_instance(&db, &app_b.id, &w1.id, 8003).await;
        add_running_instance(&db, &app_b.id, &w2.id, 8004).await;

        let on_w1 = db.list_app_instances_by_server(&w1.id).await.unwrap();
        let on_w2 = db.list_app_instances_by_server(&w2.id).await.unwrap();
        assert_eq!(on_w1.len(), 3);
        assert_eq!(on_w2.len(), 1);
        assert!(on_w1.iter().all(|i| i.server_id == w1.id));
    }

    // --- Caddy multi-upstream config (BE-259) ---

    #[tokio::test]
    async fn caddy_config_reflects_all_running_upstreams() {
        // Mirrors the deploy pipeline: a balanced route is built from the list
        // of running instances' host:port upstreams.
        let upstreams = vec![
            "w1.example.com:8001".to_string(),
            "w2.example.com:8002".to_string(),
            "localhost:8003".to_string(),
        ];
        let route = CaddyRoute::reverse_proxy_balanced(
            "app.example.com",
            None,
            &upstreams,
            "round_robin",
            "/",
        );
        let handler = &route.handle[0];
        let dialed: Vec<&str> = handler
            .upstreams
            .as_ref()
            .unwrap()
            .iter()
            .map(|u| u.dial.as_str())
            .collect();
        assert_eq!(
            dialed,
            vec![
                "w1.example.com:8001",
                "w2.example.com:8002",
                "localhost:8003"
            ]
        );
    }

    #[tokio::test]
    async fn caddy_config_carries_lb_policy_and_health_path() {
        let upstreams = vec!["a:80".to_string(), "b:80".to_string()];
        let route = CaddyRoute::reverse_proxy_balanced(
            "app.example.com",
            None,
            &upstreams,
            "ip_hash",
            "/healthz",
        );
        let handler = &route.handle[0];

        assert_eq!(
            handler.load_balancing.as_ref().unwrap()["selection_policy"]["policy"],
            caddy_lb_policy("ip_hash"),
        );
        assert_eq!(
            handler.health_checks.as_ref().unwrap()["active"]["path"],
            "/healthz",
        );
    }
}
