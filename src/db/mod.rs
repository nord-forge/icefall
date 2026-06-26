pub mod encryption;
pub mod models;
pub mod sqlite;
pub mod store;

use thiserror::Error;

// The `Database` trait is split into per-domain sub-traits under `store/`.
// Re-export the umbrella trait and every sub-trait so existing call sites can
// keep using `crate::db::Database` (and, where needed, the focused traits).
pub use store::{
    AppStore, BackupStore, Database, DatabaseStore, DeployStore, EnvironmentStore, GitHubStore,
    InfraStore, MiscStore, NetworkingStore, NotificationStore, OAuthStore, ObservabilityStore,
    ProjectStore, ProxyStore, TaskStore, TeamStore, UpdateStore, UserStore,
};

/// Bring `Database` and every domain sub-trait into scope at once. Code that
/// calls store methods on a concrete `SqliteDatabase` (rather than through
/// `&dyn Database`) needs the relevant sub-trait imported — this prelude is the
/// convenient catch-all, used mainly by tests.
pub mod prelude {
    pub use super::store::{
        AppStore, BackupStore, Database, DatabaseStore, DeployStore, EnvironmentStore, GitHubStore,
        InfraStore, MiscStore, NetworkingStore, NotificationStore, OAuthStore, ObservabilityStore,
        ProjectStore, ProxyStore, TaskStore, TeamStore, UpdateStore, UserStore,
    };
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("duplicate record: {0}")]
    Duplicate(String),
    #[error("encryption error: {0}")]
    Encryption(#[from] encryption::EncryptionError),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
