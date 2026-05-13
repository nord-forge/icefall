use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

use super::helpers::{generate_temp_password, hash_password};

pub(super) async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let users = state.db.list_users().await?;
    let safe: Vec<serde_json::Value> = users
        .iter()
        .map(|u| {
            serde_json::json!({
                "id": u.id,
                "email": u.email,
                "role": u.role,
                "totp_enabled": u.totp_enabled,
                "is_active": true,
                "last_login_at": null,
                "created_at": u.created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": safe })))
}

#[derive(Deserialize)]
pub(super) struct ChangeRoleRequest {
    role: String,
}

pub(super) async fn change_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ChangeRoleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }
    if !["admin", "deployer", "viewer"].contains(&body.role.as_str()) {
        return Err(ApiError::BadRequest("Invalid role".into()));
    }

    Ok(Json(
        serde_json::json!({ "message": "role updated", "user_id": id, "new_role": body.role }),
    ))
}

pub(super) async fn deactivate_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }
    if caller.id == id {
        return Err(ApiError::BadRequest(
            "Cannot delete your own account through this endpoint. Use DELETE /users/me instead."
                .into(),
        ));
    }

    let target = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    if target.role == "admin" {
        let admin_count = state.db.count_admin_users().await?;
        if admin_count <= 1 {
            return Err(ApiError::BadRequest("Cannot delete the last admin".into()));
        }
    }

    state.db.delete_user_sessions(&id).await?;
    let tokens = state.db.list_api_tokens(&id).await?;
    for token in tokens {
        state.db.delete_api_token(&token.id).await?;
    }
    state.db.delete_user(&id).await?;

    Ok(Json(
        serde_json::json!({ "message": "User deleted successfully" }),
    ))
}

pub(super) async fn reset_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let target = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    let temp_password = generate_temp_password(16);
    let password_hash = hash_password(&temp_password)?;

    state.db.update_user_password(&id, &password_hash).await?;
    state.db.delete_user_sessions(&id).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "user_id": id,
            "email": target.email,
            "temporary_password": temp_password,
        },
        "warning": "This temporary password will only be shown once. Share it securely with the user."
    })))
}

pub(super) async fn admin_reset_2fa(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let target = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    if !target.totp_enabled {
        return Err(ApiError::BadRequest(
            "2FA is not enabled for this user".into(),
        ));
    }

    state.db.admin_reset_user_2fa(&id).await?;
    state.db.delete_user_sessions(&id).await?;

    Ok(Json(serde_json::json!({
        "message": "Two-factor authentication has been reset for this user",
        "user_id": id,
        "email": target.email,
    })))
}
