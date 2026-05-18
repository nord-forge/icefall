use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get};
use axum::{Json, Router};

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::github::auth::generate_jwt;
use crate::github::client::GitHubClient;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/git-sources", get(list_sources))
        .route("/git-sources/{id}", delete(delete_source))
        .route("/git-sources/{id}/repos", get(list_repos))
}

async fn list_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let installations = state.db.list_github_installations().await?;
    Ok(Json(serde_json::json!({ "data": installations })))
}

async fn delete_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    state.db.delete_github_installation(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn list_repos(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    // Find the installation
    let installations = state.db.list_github_installations().await?;
    let installation = installations
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Installation {id} not found")))?;

    // Find the GitHub App linked to this installation
    let github_app = state
        .db
        .get_github_app_for_installation(installation.installation_id)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest(
                "No GitHub App linked to this installation. Please reconnect via Settings.".into(),
            )
        })?;

    // Generate JWT and get installation token
    let jwt = generate_jwt(github_app.app_id, &github_app.private_key).map_err(|e| {
        tracing::error!(
            "Failed to generate JWT for GitHub App {}: {e}",
            github_app.app_id
        );
        ApiError::Internal(Box::new(std::io::Error::other(e)))
    })?;

    let client = GitHubClient::new(&github_app.api_url);

    let token = client
        .get_installation_token(&jwt, installation.installation_id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to get installation token for installation {}: {e}",
                installation.installation_id
            );
            ApiError::Internal(Box::new(std::io::Error::other(e)))
        })?;

    let repos = client
        .list_installation_repos(&token.token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list repos: {e}");
            ApiError::Internal(Box::new(std::io::Error::other(e)))
        })?;

    Ok(Json(serde_json::json!({ "data": repos })))
}
