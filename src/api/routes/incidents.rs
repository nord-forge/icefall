use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
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
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

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
    headers: HeaderMap,
    Json(body): Json<CreateIncidentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    if user.role == "viewer" {
        return Err(ApiError::Forbidden(
            "Deployer or admin role required to create incidents".into(),
        ));
    }

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
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateStatusRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    if user.role == "viewer" {
        return Err(ApiError::Forbidden(
            "Deployer or admin role required to update incidents".into(),
        ));
    }

    state.db.update_incident_status(&id, &body.status).await?;
    Ok(Json(serde_json::json!({ "message": "updated" })))
}

#[derive(Deserialize)]
struct AddNoteRequest {
    content: String,
}

async fn add_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<AddNoteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let note = state
        .db
        .add_incident_note(&id, &body.content, Some(&user.id))
        .await?;
    Ok(Json(serde_json::json!({ "data": note })))
}
