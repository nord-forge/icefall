use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

use super::helpers::verify_password;

pub(super) async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    Ok(Json(
        serde_json::json!({ "data": { "id": user.id, "email": user.email, "role": user.role, "totp_enabled": user.totp_enabled, "created_at": user.created_at } }),
    ))
}

#[derive(Deserialize)]
pub(super) struct DeleteAccountRequest {
    password: String,
}

pub(super) async fn delete_own_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DeleteAccountRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if !verify_password(&body.password, &user.password_hash) {
        return Err(ApiError::BadRequest("Password is incorrect".into()));
    }

    if user.role == "admin" {
        let admin_count = state.db.count_admin_users().await?;
        if admin_count <= 1 {
            return Err(ApiError::BadRequest(
                "Cannot delete the last admin account".into(),
            ));
        }
    }

    state.db.delete_user_sessions(&user.id).await?;
    let tokens = state.db.list_api_tokens(&user.id).await?;
    for token in tokens {
        state.db.delete_api_token(&token.id).await?;
    }
    state.db.delete_user(&user.id).await?;

    Ok(Json(
        serde_json::json!({ "message": "Account deleted successfully" }),
    ))
}
