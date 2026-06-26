use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait TaskStore: Send + Sync {
    // Scheduled tasks
    async fn list_scheduled_tasks(&self, app_id: &str) -> Result<Vec<ScheduledTask>, DbError>;
    async fn create_scheduled_task(
        &self,
        task: &NewScheduledTask,
    ) -> Result<ScheduledTask, DbError>;
    async fn update_scheduled_task_enabled(&self, id: &str, enabled: bool) -> Result<(), DbError>;
    async fn delete_scheduled_task(&self, id: &str) -> Result<(), DbError>;
    async fn list_all_enabled_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>, DbError>;
    async fn create_task_execution(
        &self,
        task_id: &str,
        status: &str,
        output: Option<&str>,
    ) -> Result<ScheduledTaskExecution, DbError>;
    async fn update_task_execution(
        &self,
        id: &str,
        status: &str,
        output: Option<&str>,
    ) -> Result<(), DbError>;
    async fn list_task_executions(
        &self,
        task_id: &str,
        limit: i64,
    ) -> Result<Vec<ScheduledTaskExecution>, DbError>;

    // Container cleanup executions
    async fn create_cleanup_execution(
        &self,
        server_id: &str,
    ) -> Result<ContainerCleanupExecution, DbError>;
    async fn update_cleanup_execution(
        &self,
        id: &str,
        status: &str,
        space_reclaimed: Option<i64>,
        images: i32,
        volumes: i32,
        networks: i32,
    ) -> Result<(), DbError>;
    async fn list_cleanup_executions(
        &self,
        server_id: &str,
        limit: i64,
    ) -> Result<Vec<ContainerCleanupExecution>, DbError>;

    // Cleanup schedule
    async fn get_cleanup_schedule(&self) -> Result<Option<CleanupSchedule>, DbError>;
    async fn upsert_cleanup_schedule(
        &self,
        schedule: &CleanupSchedule,
    ) -> Result<CleanupSchedule, DbError>;

    // Cleanup runs
    async fn create_cleanup_run(&self) -> Result<CleanupRun, DbError>;
    async fn finish_cleanup_run(
        &self,
        id: &str,
        status: &str,
        freed_bytes: i64,
        removed_items: i64,
        error: Option<&str>,
        details: Option<&str>,
    ) -> Result<(), DbError>;
    async fn list_cleanup_runs(&self, limit: i64) -> Result<Vec<CleanupRun>, DbError>;
}
