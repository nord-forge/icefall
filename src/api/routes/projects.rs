use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::{NewProject, UpdateProject};

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
    description: Option<String>,
    color: Option<String>,
}

#[derive(Deserialize)]
struct UpdateProjectRequest {
    name: Option<String>,
    description: Option<Option<String>>,
    color: Option<Option<String>>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
}

async fn list_projects(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let (projects, apps, dbs) = tokio::join!(
        state.db.list_projects(),
        state.db.list_apps(),
        state.db.list_managed_dbs()
    );
    let projects = projects?;
    let apps = apps?;
    let dbs = dbs?;

    let mut app_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for app in &apps {
        if let Some(ref pid) = app.project_id {
            *app_counts.entry(pid).or_default() += 1;
        }
    }
    let mut db_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for db in &dbs {
        if let Some(ref pid) = db.project_id {
            *db_counts.entry(pid).or_default() += 1;
        }
    }

    let result: Vec<_> = projects
        .iter()
        .map(|project| {
            serde_json::json!({
                "id": project.id,
                "name": project.name,
                "description": project.description,
                "color": project.color,
                "app_count": app_counts.get(project.id.as_str()).copied().unwrap_or(0),
                "database_count": db_counts.get(project.id.as_str()).copied().unwrap_or(0),
                "created_at": project.created_at,
                "updated_at": project.updated_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": result })))
}

async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Project name must not be empty".to_string(),
        ));
    }

    let project = state
        .db
        .create_project(&NewProject {
            name: body.name,
            description: body.description,
            color: body.color,
        })
        .await?;

    Ok(Json(serde_json::json!({ "data": project })))
}

async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let project = state
        .db
        .get_project(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Project '{id}' not found")))?;

    let apps = state.db.list_apps_by_project(&id).await?;
    let databases = state.db.list_managed_dbs_by_project(&id).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": project.id,
            "name": project.name,
            "description": project.description,
            "color": project.color,
            "created_at": project.created_at,
            "updated_at": project.updated_at,
            "apps": apps,
            "databases": databases,
        }
    })))
}

async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProjectRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Some(ref name) = body.name {
        if name.trim().is_empty() {
            return Err(ApiError::BadRequest(
                "Project name must not be empty".to_string(),
            ));
        }
    }

    let project = state
        .db
        .update_project(
            &id,
            &UpdateProject {
                name: body.name,
                description: body.description,
                color: body.color,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": project })))
}

async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.delete_project(&id).await?;
    Ok(Json(
        serde_json::json!({ "message": "deleted", "detail": "All resources have been unassigned from this project." }),
    ))
}
