use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get};
use axum::{Json, Router};

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::github::client::GitHubClient;
use crate::github::token::get_valid_installation_token;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/git-sources", get(list_sources))
        .route("/git-sources/{id}", delete(delete_source))
        .route("/git-sources/{id}/repos", get(list_repos))
        .route("/git-sources/{id}/branches", get(list_branches))
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

    // Cached-or-refreshed installation token (avoids minting a fresh JWT per call).
    let resolved = get_valid_installation_token(&state.db, &id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to resolve installation token for {id}: {e}");
            ApiError::BadRequest(e)
        })?;

    let client = GitHubClient::new(&resolved.api_url);
    let repos = client
        .list_installation_repos(&resolved.token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list repos: {e}");
            ApiError::Internal(Box::new(std::io::Error::other(e)))
        })?;

    Ok(Json(serde_json::json!({ "data": repos })))
}

#[derive(serde::Deserialize)]
struct BranchQuery {
    repo: String,
}

/// GET /git-sources/{id}/branches?repo=owner/name — branch names for a repo.
async fn list_branches(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<BranchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let (owner, repo) = q
        .repo
        .split_once('/')
        .ok_or_else(|| ApiError::BadRequest("repo must be in owner/name form".into()))?;

    let resolved = get_valid_installation_token(&state.db, &id)
        .await
        .map_err(ApiError::BadRequest)?;

    let client = GitHubClient::new(&resolved.api_url);
    let branches = client
        .list_repo_branches(&resolved.token, owner, repo)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list branches for {}: {e}", q.repo);
            ApiError::Internal(Box::new(std::io::Error::other(e)))
        })?;

    Ok(Json(serde_json::json!({ "data": branches })))
}
