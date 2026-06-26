//! The `Database` abstraction, split into cohesive per-domain sub-traits.
//!
//! `Database` itself is just the union of every domain trait plus migrations.
//! Because the domain traits are *supertraits* of `Database`, every method is
//! reachable through `&dyn Database` without importing the individual traits —
//! so call sites keep using `crate::db::Database` unchanged. Each sub-trait
//! lives in its own file here; the `SqliteDatabase` impls live in
//! `crate::db::sqlite::<domain>`.

mod app;
mod backup;
mod cleanup_task;
mod database;
mod deploy;
mod environment;
mod github;
mod infra;
mod misc;
mod networking;
mod notification;
mod oauth;
mod observability;
mod project;
mod proxy;
mod team;
mod update;
mod user;

pub use app::AppStore;
pub use backup::BackupStore;
pub use cleanup_task::TaskStore;
pub use database::DatabaseStore;
pub use deploy::DeployStore;
pub use environment::EnvironmentStore;
pub use github::GitHubStore;
pub use infra::InfraStore;
pub use misc::MiscStore;
pub use networking::NetworkingStore;
pub use notification::NotificationStore;
pub use oauth::OAuthStore;
pub use observability::ObservabilityStore;
pub use project::ProjectStore;
pub use proxy::ProxyStore;
pub use team::TeamStore;
pub use update::UpdateStore;
pub use user::UserStore;

use async_trait::async_trait;

use crate::db::DbError;

/// The full database interface: the union of all domain sub-traits.
///
/// Implementors only need to implement each sub-trait; the blanket bound here
/// composes them. Callers depend on `Database` and reach any domain method via
/// dynamic dispatch.
#[async_trait]
pub trait Database:
    ProjectStore
    + AppStore
    + ProxyStore
    + EnvironmentStore
    + DeployStore
    + DatabaseStore
    + NetworkingStore
    + TaskStore
    + InfraStore
    + GitHubStore
    + ObservabilityStore
    + UserStore
    + NotificationStore
    + BackupStore
    + OAuthStore
    + UpdateStore
    + MiscStore
    + TeamStore
    + Send
    + Sync
    + 'static
{
    // Migrations
    async fn run_migrations(&self) -> Result<(), DbError>;
}
