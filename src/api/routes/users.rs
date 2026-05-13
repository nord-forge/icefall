use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/invite", post(invite_user))
        .route("/users/me", get(get_me).delete(delete_own_account))
        .route("/users/{id}/role", put(change_role))
        .route("/users/{id}", delete(deactivate_user))
        .route("/users/{id}/reset-password", post(reset_password))
        .route("/users/{id}/2fa", delete(admin_reset_2fa))
        .route("/users/accept-invite", post(accept_invite))
        .route("/tokens", get(list_tokens).post(create_token))
        .route("/tokens/{id}", delete(revoke_token))
}

async fn list_users(
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

async fn get_me(
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
struct DeleteAccountRequest {
    password: String,
}

async fn delete_own_account(
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

    // Cannot delete the last admin
    if user.role == "admin" {
        let admin_count = state.db.count_admin_users().await?;
        if admin_count <= 1 {
            return Err(ApiError::BadRequest(
                "Cannot delete the last admin account".into(),
            ));
        }
    }

    // Delete all sessions, tokens, then the user
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

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[derive(Deserialize)]
struct InviteRequest {
    email: String,
    role: Option<String>,
}

async fn invite_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let role = body.role.unwrap_or_else(|| "viewer".to_string());
    if !["admin", "deployer", "viewer"].contains(&role.as_str()) {
        return Err(ApiError::BadRequest(
            "Role must be admin, deployer, or viewer".into(),
        ));
    }

    let token = generate_random_hex(48);
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(7))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let invitation = state
        .db
        .create_invitation(&body.email, &role, &token, &expires_at)
        .await?;

    let base_url = state
        .config
        .base_domain
        .as_deref()
        .map(|d| format!("https://{d}"))
        .unwrap_or_default();
    let invite_url = format!("{base_url}/auth/accept-invite?token={token}");

    // Best-effort: send invite email if SMTP is configured
    if let Ok(channels) = state.db.list_notification_channels().await {
        if let Some(smtp_channel) = channels.iter().find(|c| c.channel_type == "smtp") {
            let details = serde_json::json!({
                "email": &body.email,
                "role": &role,
                "invite_url": &invite_url,
            });
            let _ = crate::api::routes::notifications::dispatch_notification(
                &smtp_channel.channel_type,
                &smtp_channel.config,
                "user.invited",
                &format!("You've been invited to Icefall as {role}"),
                &details,
            )
            .await;
        }
    }

    Ok(Json(serde_json::json!({
        "data": invitation,
        "invite_url": invite_url,
    })))
}

#[derive(Deserialize)]
struct AcceptInviteRequest {
    token: String,
    password: String,
}

async fn accept_invite(
    State(state): State<AppState>,
    Json(body): Json<AcceptInviteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let invitation = state
        .db
        .get_invitation_by_token(&body.token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Invalid or expired invitation".into()))?;

    if invitation.expires_at < crate::db::models::now_iso8601() {
        return Err(ApiError::BadRequest("Invitation has expired".into()));
    }
    if body.password.len() < 12 {
        return Err(ApiError::BadRequest(
            "Password must be at least 12 characters".into(),
        ));
    }

    let password_hash = hash_password(&body.password)?;
    let user = state
        .db
        .create_user(&NewUser {
            email: invitation.email,
            password_hash,
            role: invitation.role,
        })
        .await?;

    state.db.delete_invitation(&invitation.id).await?;

    Ok(Json(
        serde_json::json!({ "data": { "id": user.id, "email": user.email, "role": user.role } }),
    ))
}

#[derive(Deserialize)]
struct ChangeRoleRequest {
    role: String,
}

async fn change_role(
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

async fn deactivate_user(
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

    // Verify target user exists
    let target = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    // Cannot delete the last admin
    if target.role == "admin" {
        let admin_count = state.db.count_admin_users().await?;
        if admin_count <= 1 {
            return Err(ApiError::BadRequest("Cannot delete the last admin".into()));
        }
    }

    // Delete sessions, tokens, then the user
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

// --- Password Reset (Admin) ---

async fn reset_password(
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

    // Verify target user exists
    let target = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    // Generate a secure temporary password (16 chars alphanumeric)
    let temp_password = generate_temp_password(16);
    let password_hash = hash_password(&temp_password)?;

    state.db.update_user_password(&id, &password_hash).await?;

    // Invalidate all existing sessions for the target user
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

// --- Admin 2FA Reset ---

async fn admin_reset_2fa(
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

    // Verify target user exists
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

    // Invalidate sessions so they must log in fresh
    state.db.delete_user_sessions(&id).await?;

    Ok(Json(serde_json::json!({
        "message": "Two-factor authentication has been reset for this user",
        "user_id": id,
        "email": target.email,
    })))
}

// --- API Tokens ---

#[derive(Deserialize)]
struct CreateTokenRequest {
    name: String,
    expires_at: Option<String>,
}

async fn list_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let tokens = state.db.list_api_tokens(&user.id).await?;
    let safe: Vec<serde_json::Value> = tokens.iter().map(|t| serde_json::json!({
        "id": t.id, "name": t.name, "last_used_at": t.last_used_at, "expires_at": t.expires_at, "created_at": t.created_at,
    })).collect();

    Ok(Json(serde_json::json!({ "data": safe })))
}

async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let raw_token = format!("icefall_{}", generate_random_hex(48));
    let token_hash = sha256_hex(&raw_token);

    let token = state
        .db
        .create_api_token(
            &user.id,
            &body.name,
            &token_hash,
            body.expires_at.as_deref(),
        )
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "id": token.id, "name": token.name, "token": raw_token },
        "warning": "This token will only be shown once. Store it securely."
    })))
}

async fn revoke_token(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    state.db.delete_api_token(&id).await?;
    Ok(Json(serde_json::json!({ "message": "token revoked" })))
}

fn generate_temp_password(len: usize) -> String {
    use rand::Rng;
    let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789!@#$%";
    let mut rng = rand::rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}

fn generate_random_hex(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..len)
        .map(|_| format!("{:02x}", rng.random::<u8>()))
        .collect()
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?;
    Ok(hash.to_string())
}

fn sha256_hex(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
