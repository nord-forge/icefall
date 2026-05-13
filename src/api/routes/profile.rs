use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::{authenticate_from_headers, extract_session_id};
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users/me/password", put(change_password))
        .route("/users/me/email", put(change_email))
        .route(
            "/users/me/sessions",
            get(list_sessions).delete(revoke_all_sessions),
        )
        .route(
            "/users/me/preferences",
            get(get_preferences).put(update_preferences),
        )
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct ChangeEmailRequest {
    email: String,
    password: String,
}

async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if !verify_password(&body.current_password, &user.password_hash) {
        return Err(ApiError::BadRequest("Current password is incorrect".into()));
    }

    if body.new_password.len() < 12 {
        return Err(ApiError::BadRequest(
            "New password must be at least 12 characters".into(),
        ));
    }

    let new_hash = hash_password(&body.new_password)?;
    state.db.update_user_password(&user.id, &new_hash).await?;

    // Revoke all sessions except the current one so the user stays logged in
    if let Some(session_id) = extract_session_id(&headers) {
        state
            .db
            .delete_user_sessions_except(&user.id, &session_id)
            .await?;
    } else {
        state.db.delete_user_sessions(&user.id).await?;
    }

    Ok(Json(
        serde_json::json!({ "message": "Password changed successfully" }),
    ))
}

async fn change_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChangeEmailRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if !verify_password(&body.password, &user.password_hash) {
        return Err(ApiError::BadRequest("Password is incorrect".into()));
    }

    let email = body.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::BadRequest("Invalid email address".into()));
    }

    state.db.update_user_email(&user.id, &email).await?;

    Ok(Json(serde_json::json!({
        "message": "Email updated successfully",
        "data": { "email": email }
    })))
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let current_session_id = extract_session_id(&headers);
    let sessions = state.db.list_user_sessions(&user.id).await?;
    let now = crate::db::models::now_iso8601();

    let data: Vec<serde_json::Value> = sessions
        .into_iter()
        .filter(|s| s.expires_at > now)
        .map(|s| {
            let is_current = current_session_id
                .as_ref()
                .is_some_and(|cid| cid == &s.id);
            serde_json::json!({
                "id": s.id,
                "created_at": s.created_at,
                "expires_at": s.expires_at,
                "is_current": is_current,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": data })))
}

async fn revoke_all_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    // Keep the current session active so the user doesn't get logged out
    if let Some(session_id) = extract_session_id(&headers) {
        state
            .db
            .delete_user_sessions_except(&user.id, &session_id)
            .await?;
    } else {
        state.db.delete_user_sessions(&user.id).await?;
    }

    Ok(Json(
        serde_json::json!({ "message": "All other sessions have been revoked" }),
    ))
}

async fn get_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let preferences = state.db.get_user_preferences(&user.id).await?;

    Ok(Json(serde_json::json!({ "data": preferences })))
}

async fn update_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if !body.is_object() {
        return Err(ApiError::BadRequest(
            "Preferences must be a JSON object".into(),
        ));
    }

    // Merge incoming preferences into existing ones (partial update)
    let mut existing = state.db.get_user_preferences(&user.id).await?;
    if let (Some(existing_obj), Some(incoming_obj)) = (existing.as_object_mut(), body.as_object()) {
        for (key, value) in incoming_obj {
            existing_obj.insert(key.clone(), value.clone());
        }
    }

    state
        .db
        .update_user_preferences(&user.id, &existing)
        .await?;

    Ok(Json(serde_json::json!({ "data": existing })))
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}
