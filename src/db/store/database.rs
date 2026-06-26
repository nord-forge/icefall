use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait DatabaseStore: Send + Sync {
    // Managed Databases
    async fn create_managed_db(&self, db: &NewManagedDatabase) -> Result<ManagedDatabase, DbError>;
    async fn list_managed_dbs(&self) -> Result<Vec<ManagedDatabase>, DbError>;
    async fn list_managed_dbs_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ManagedDatabase>, DbError>;
    async fn update_managed_db_credentials(
        &self,
        id: &str,
        credentials_json: &str,
        container_id: &str,
    ) -> Result<(), DbError>;
    async fn delete_managed_db(&self, id: &str) -> Result<(), DbError>;

    // Database restore
    async fn create_restore_record(
        &self,
        database_id: &str,
        source_type: &str,
        source_ref: Option<&str>,
    ) -> Result<DatabaseRestoreRecord, DbError>;
    async fn update_restore_record(
        &self,
        id: &str,
        status: &str,
        output: Option<&str>,
    ) -> Result<(), DbError>;
    async fn list_restore_history(
        &self,
        database_id: &str,
        limit: i64,
    ) -> Result<Vec<DatabaseRestoreRecord>, DbError>;

    // Database SSL
    async fn update_database_ssl(
        &self,
        id: &str,
        ssl_enabled: bool,
        ssl_mode: Option<&str>,
    ) -> Result<(), DbError>;
    async fn store_database_certs(
        &self,
        id: &str,
        ca_cert: &str,
        cert: &str,
        key: &str,
        expires_at: &str,
    ) -> Result<(), DbError>;
}
