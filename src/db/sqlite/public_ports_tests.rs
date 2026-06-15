//! IF-172: data-layer tests for public-port allocation — lowest-free selection,
//! release, and range exhaustion. Docker/Caddy-independent.

#[cfg(test)]
mod public_ports {
    use std::sync::Arc;

    use sqlx::sqlite::SqlitePoolOptions;

    use crate::db::encryption::Encryptor;
    use crate::db::sqlite::SqliteDatabase;
    use crate::db::{Database, DbError};

    async fn setup_db() -> SqliteDatabase {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect to in-memory SQLite");

        sqlx::migrate!("src/db/migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        let encryptor = Arc::new(Encryptor::new(&Encryptor::generate_key()));
        SqliteDatabase::new_with_pool(pool, encryptor)
    }

    #[tokio::test]
    async fn allocates_lowest_free_port_in_range() {
        let db = setup_db().await;

        // First allocation takes the low bound.
        let a = db
            .allocate_free_public_port("database", "db-a", 10000, 10002, None)
            .await
            .expect("allocate a");
        assert_eq!(a.port, 10000);

        // Second takes the next free port, not a duplicate.
        let b = db
            .allocate_free_public_port("database", "db-b", 10000, 10002, None)
            .await
            .expect("allocate b");
        assert_eq!(b.port, 10001);

        // Releasing the low port frees it for reuse below the high port.
        db.release_public_port("db-a").await.expect("release a");
        let c = db
            .allocate_free_public_port("database", "db-c", 10000, 10002, None)
            .await
            .expect("allocate c");
        assert_eq!(c.port, 10000);
    }

    #[tokio::test]
    async fn exhausted_range_is_rejected() {
        let db = setup_db().await;

        // A single-port range: first succeeds, second has nowhere to go.
        db.allocate_free_public_port("database", "only", 11000, 11000, None)
            .await
            .expect("allocate only");

        let err = db
            .allocate_free_public_port("database", "overflow", 11000, 11000, None)
            .await
            .expect_err("range is full");
        assert!(matches!(err, DbError::InvalidInput(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn whitelist_is_persisted_and_retrievable() {
        let db = setup_db().await;

        db.allocate_free_public_port("database", "wl", 12000, 12010, Some("1.2.3.4,10.0.0.0/8"))
            .await
            .expect("allocate with whitelist");

        let fetched = db
            .get_public_port("wl")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(fetched.port, 12000);
        assert_eq!(fetched.ip_whitelist.as_deref(), Some("1.2.3.4,10.0.0.0/8"));
    }
}
