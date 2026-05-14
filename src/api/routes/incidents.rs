use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewIncident;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/incidents", get(list_incidents).post(create_incident))
        .route("/incidents/{id}/status", put(update_status))
        .route("/incidents/{id}/notes", post(add_note))
}

async fn list_incidents(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let incidents = state.db.list_incidents(50).await?;
    Ok(Json(serde_json::json!({ "data": incidents })))
}

#[derive(Deserialize)]
struct CreateIncidentRequest {
    title: String,
    severity: Option<String>,
    affected_apps: Option<String>,
    affected_servers: Option<String>,
}

async fn create_incident(
    State(state): State<AppState>,
    Json(body): Json<CreateIncidentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let incident = state
        .db
        .create_incident(&NewIncident {
            title: body.title,
            severity: body.severity.unwrap_or_else(|| "minor".into()),
            affected_apps: body.affected_apps,
            affected_servers: body.affected_servers,
        })
        .await?;
    Ok(Json(serde_json::json!({ "data": incident })))
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: String,
}

async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateStatusRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.db.update_incident_status(&id, &body.status).await?;
    Ok(Json(serde_json::json!({ "message": "updated" })))
}

#[derive(Deserialize)]
struct AddNoteRequest {
    content: String,
    author_id: Option<String>,
}

async fn add_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AddNoteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let note = state
        .db
        .add_incident_note(&id, &body.content, body.author_id.as_deref())
        .await?;
    Ok(Json(serde_json::json!({ "data": note })))
}
