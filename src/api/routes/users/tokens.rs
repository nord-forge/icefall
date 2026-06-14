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
    /// Optional ability scopes. Omitted/empty = full access (null in storage).
    abilities: Option<Vec<String>>,
}

/// Parse a token's stored abilities JSON into a list for API responses.
fn abilities_list(stored: &Option<String>) -> Vec<String> {
    stored
        .as_deref()
        .and_then(|j| serde_json::from_str::<Vec<String>>(j).ok())
        .unwrap_or_default()
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
        "id": t.id, "name": t.name, "abilities": abilities_list(&t.abilities), "last_used_at": t.last_used_at, "expires_at": t.expires_at, "created_at": t.created_at,
    })).collect();

    Ok(Json(
        serde_json::json!({ "data": safe, "available_abilities": crate::api::abilities::ALL_ABILITIES }),
    ))
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

    // Empty/omitted abilities → null (full access). Otherwise keep only valid
    // scopes and store as a JSON array.
    let abilities_json = match body.abilities {
        Some(ref list) if !list.is_empty() => {
            let valid = crate::api::abilities::sanitize_abilities(list);
            if valid.is_empty() {
                return Err(ApiError::BadRequest(
                    "No valid abilities provided. Omit the field for full access.".into(),
                ));
            }
            Some(serde_json::to_string(&valid).unwrap_or_default())
        }
        _ => None,
    };

    let token = state
        .db
        .create_api_token(
            &auth.user.id,
            &body.name,
            &token_hash,
            body.expires_at.as_deref(),
            auth.team_id.as_deref(),
            abilities_json.as_deref(),
        )
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "id": token.id, "name": token.name, "token": raw_token, "abilities": abilities_list(&token.abilities) },
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
