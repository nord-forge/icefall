use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewScheduledTask;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/apps/{id}/scheduled-tasks",
            get(list_tasks).post(create_task),
        )
        .route(
            "/apps/{id}/scheduled-tasks/{task_id}",
            axum::routing::delete(delete_task),
        )
        .route(
            "/apps/{id}/scheduled-tasks/{task_id}/toggle",
            post(toggle_task),
        )
        .route("/apps/{id}/scheduled-tasks/{task_id}/run", post(run_task))
        .route(
            "/apps/{id}/scheduled-tasks/{task_id}/executions",
            get(list_executions),
        )
}

async fn list_tasks(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let tasks = state.db.list_scheduled_tasks(&app_id).await?;
    Ok(Json(serde_json::json!({ "data": tasks })))
}

#[derive(Deserialize)]
struct CreateTaskRequest {
    name: String,
    command: String,
    cron_expression: String,
    timeout_seconds: Option<i32>,
    container_name: Option<String>,
}

async fn create_task(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
    Json(body): Json<CreateTaskRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let task = state
        .db
        .create_scheduled_task(&NewScheduledTask {
            app_id,
            name: body.name,
            command: body.command,
            cron_expression: body.cron_expression,
            timeout_seconds: body.timeout_seconds,
            container_name: body.container_name,
        })
        .await?;
    Ok(Json(serde_json::json!({ "data": task })))
}

async fn delete_task(
    State(state): State<AppState>,
    Path((_app_id, task_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.delete_scheduled_task(&task_id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

#[derive(Deserialize)]
struct ToggleRequest {
    enabled: bool,
}

async fn toggle_task(
    State(state): State<AppState>,
    Path((_app_id, task_id)): Path<(String, String)>,
    Json(body): Json<ToggleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .update_scheduled_task_enabled(&task_id, body.enabled)
        .await?;
    Ok(Json(serde_json::json!({ "message": "updated" })))
}

async fn run_task(
    State(state): State<AppState>,
    Path((app_id, task_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let tasks = state.db.list_scheduled_tasks(&app_id).await?;
    let task = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| ApiError::NotFound(format!("task {task_id}")))?;

    let execution = state
        .db
        .create_task_execution(&task_id, "running", None)
        .await?;

    let app = state
        .db
        .get_app(&app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {app_id}")))?;

    let cmd = task.command.clone();
    let exec_id = execution.id.clone();
    let docker = state.docker.clone();
    let db = state.db.clone();

    tokio::spawn(async move {
        let deploys = db.list_deploys(&app.id, 1).await.unwrap_or_default();
        let Some(deploy) = deploys.first() else {
            let _ = db
                .update_task_execution(&exec_id, "failed", Some("No running deploy"))
                .await;
            return;
        };
        let Some(ref container_id) = deploy.container_id else {
            let _ = db
                .update_task_execution(&exec_id, "failed", Some("No container ID"))
                .await;
            return;
        };

        let cmd_parts = vec!["sh".to_string(), "-c".to_string(), cmd];
        match docker.exec_in_container(container_id, &cmd_parts).await {
            Ok(output) => {
                let truncated = if output.len() > 1_000_000 {
                    format!("{}... (truncated)", &output[..1_000_000])
                } else {
                    output
                };
                let _ = db
                    .update_task_execution(&exec_id, "success", Some(&truncated))
                    .await;
            }
            Err(e) => {
                let _ = db
                    .update_task_execution(&exec_id, "failed", Some(&e.to_string()))
                    .await;
            }
        }
    });

    Ok(Json(serde_json::json!({ "data": execution })))
}

async fn list_executions(
    State(state): State<AppState>,
    Path((_app_id, task_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let execs = state.db.list_task_executions(&task_id, 50).await?;
    Ok(Json(serde_json::json!({ "data": execs })))
}
