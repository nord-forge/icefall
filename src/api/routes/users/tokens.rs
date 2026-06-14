use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

use super::helpers::{generate_random_hex, sha256_hex};

#[derive(Deserialize)]
pub(super) struct CreateTokenRequest {
    name: String,
    expires_at: Option<String>,
}

pub(super) async fn list_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let tokens = state.db.list_api_tokens(&user.id).await?;
    let safe: Vec<serde_json::Value> = tokens.iter().map(|t| serde_json::json!({
        "id": t.id, "name": t.name, "last_used_at": t.last_used_at, "expires_at": t.expires_at, "created_at": t.created_at,
    })).collect();

    Ok(Json(serde_json::json!({ "data": safe })))
}

pub(super) async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = crate::api::routes::auth::authenticate_with_team(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let raw_token = format!("icefall_{}", generate_random_hex(48));
    let token_hash = sha256_hex(&raw_token);

    let token = state
        .db
        .create_api_token(
            &auth.user.id,
            &body.name,
            &token_hash,
            body.expires_at.as_deref(),
            auth.team_id.as_deref(),
        )
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "id": token.id, "name": token.name, "token": raw_token },
        "warning": "This token will only be shown once. Store it securely."
    })))
}

pub(super) async fn revoke_token(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    // Verify the token belongs to the caller before deleting; a 404 (not 403)
    // avoids confirming the id exists for another user.
    let owns_token = state
        .db
        .list_api_tokens(&user.id)
        .await?
        .iter()
        .any(|t| t.id == id);
    if !owns_token {
        return Err(ApiError::NotFound("token not found".into()));
    }

    state.db.delete_api_token(&id).await?;
    Ok(Json(serde_json::json!({ "message": "token revoked" })))
}
