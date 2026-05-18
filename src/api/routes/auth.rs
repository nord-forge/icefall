use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/password", put(change_password))
        .route("/auth/setup", get(setup_status).post(setup_admin))
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct SetupRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn setup_status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let users = state.db.list_users().await?;
    Ok(Json(serde_json::json!({
        "needs_setup": users.is_empty(),
        "user_count": users.len(),
    })))
}

async fn setup_admin(
    State(state): State<AppState>,
    Json(body): Json<SetupRequest>,
) -> Result<axum::response::Response, ApiError> {
    if body.password.len() < 12 {
        return Err(ApiError::BadRequest(
            "Password must be at least 12 characters".into(),
        ));
    }

    let password_hash = hash_password(&body.password)?;
    // Atomic guard against a setup race (audit H8): create_first_admin
    // inserts only if no user exists, otherwise returns Duplicate.
    let user = state
        .db
        .create_first_admin(&NewUser {
            email: body.email,
            password_hash,
            role: "admin".to_string(),
        })
        .await
        .map_err(|e| match e {
            crate::db::DbError::Duplicate(_) => {
                ApiError::Conflict("Admin account already exists".into())
            }
            other => other.into(),
        })?;

    let session = create_session_for_user(&state, &user.id).await?;

    let body = serde_json::json!({
        "data": {
            "user": { "id": user.id, "email": user.email, "role": user.role },
            "session_id": session.id,
        }
    });
    let cookie = session_cookie(&session.id, state.config.base_domain.as_deref());
    let mut headers = HeaderMap::new();
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&cookie).map_err(ApiError::internal)?,
    );
    Ok((headers, Json(body)).into_response())
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Result<axum::response::Response, ApiError> {
    // Per-IP rate limit before any DB work — blocks credential brute-force
    // (audit H1). 5 attempts / 15 min.
    let ip = crate::api::rate_limit::client_ip(&headers);
    if !crate::api::rate_limit::LOGIN.check(&ip).await {
        return Err(ApiError::TooManyRequests(
            "Too many login attempts. Try again later.".into(),
        ));
    }

    let user = state
        .db
        .get_user_by_email(&body.email)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Invalid email or password".into()))?;

    if !verify_password(&body.password, &user.password_hash) {
        return Err(ApiError::BadRequest("Invalid email or password".into()));
    }

    // If 2FA is enabled, don't create a session yet — require 2FA validation
    // first. Hand the client an opaque, single-use challenge token instead of
    // the user_id: the user_id must never leak (audit C2, L3), and the
    // /2fa/validate endpoint must not be drivable with an attacker-chosen id.
    if user.totp_enabled {
        let challenge = crate::api::routes::two_factor_challenge::issue_challenge(&user.id).await;
        let body = serde_json::json!({
            "requires_2fa": true,
            "challenge": challenge,
        });
        return Ok(Json(body).into_response());
    }

    let session = create_session_for_user(&state, &user.id).await?;

    let body = serde_json::json!({
        "data": {
            "user": { "id": user.id, "email": user.email, "role": user.role, "totp_enabled": user.totp_enabled },
            "session_id": session.id,
        }
    });
    let cookie = session_cookie(&session.id, state.config.base_domain.as_deref());
    let mut headers = HeaderMap::new();
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&cookie).map_err(ApiError::internal)?,
    );
    Ok((headers, Json(body)).into_response())
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Some(session_id) = extract_session_id(&headers) {
        state.db.delete_session(&session_id).await?;
    }
    Ok(Json(serde_json::json!({ "message": "logged out" })))
}

async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

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
    state.db.delete_user_sessions(&user.id).await?;

    Ok(Json(serde_json::json!({ "message": "password changed" })))
}

async fn create_session_for_user(
    state: &AppState,
    user_id: &str,
) -> Result<crate::db::models::Session, ApiError> {
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(7))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    state
        .db
        .create_session(user_id, &expires_at)
        .await
        .map_err(ApiError::internal)
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::internal(std::io::Error::other(e.to_string())))?;
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

pub fn extract_session_id(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("authorization") {
        let auth_str = auth.to_str().ok()?;
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    if let Some(cookie) = headers.get("cookie") {
        let cookie_str = cookie.to_str().ok()?;
        for pair in cookie_str.split(';') {
            let pair = pair.trim();
            if let Some(value) = pair.strip_prefix("icefall_session=") {
                return Some(value.to_string());
            }
        }
    }

    None
}

pub async fn authenticate_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<crate::db::models::User>, ApiError> {
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer icefall_") {
                let full_token = format!("icefall_{token}");
                let token_hash = sha256_hex(&full_token);
                if let Some(api_token) = state.db.get_api_token_by_hash(&token_hash).await? {
                    if let Some(ref exp) = api_token.expires_at {
                        if exp < &crate::db::models::now_iso8601() {
                            return Ok(None);
                        }
                    }
                    let _ = state.db.update_token_last_used(&api_token.id).await;
                    let users = state.db.list_users().await?;
                    return Ok(users.into_iter().find(|u| u.id == api_token.user_id));
                }
            }
        }
    }

    let Some(session_id) = extract_session_id(headers) else {
        return Ok(None);
    };

    let Some(session) = state.db.get_session(&session_id).await? else {
        return Ok(None);
    };

    if session.expires_at < crate::db::models::now_iso8601() {
        state.db.delete_session(&session_id).await?;
        return Ok(None);
    }

    let users = state.db.list_users().await?;
    Ok(users.into_iter().find(|u| u.id == session.user_id))
}

pub struct AuthContext {
    pub user: crate::db::models::User,
    pub session_id: Option<String>,
    pub team_id: Option<String>,
    pub team_role: Option<String>,
}

pub async fn authenticate_with_team(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<AuthContext>, ApiError> {
    let user = match authenticate_from_headers(state, headers).await? {
        Some(u) => u,
        None => return Ok(None),
    };

    let session_id = extract_session_id(headers);

    // Check if authenticating via API token — use token's team_id
    let token_team_id = if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer icefall_") {
                let full_token = format!("icefall_{token}");
                let token_hash = sha256_hex(&full_token);
                state
                    .db
                    .get_api_token_by_hash(&token_hash)
                    .await?
                    .and_then(|t| t.team_id)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Priority: token team_id > session active_team_id > first team
    let team_id = if let Some(tid) = token_team_id {
        Some(tid)
    } else if let Some(ref sid) = session_id {
        if let Some(session) = state.db.get_session(sid).await? {
            session.active_team_id
        } else {
            None
        }
    } else {
        None
    };

    let team_id = match team_id {
        Some(tid) => Some(tid),
        None => {
            let teams = state.db.list_teams_for_user(&user.id).await?;
            teams.first().map(|t| t.id.clone())
        }
    };

    // Get team role
    let team_role = if let Some(ref tid) = team_id {
        state
            .db
            .get_team_membership(tid, &user.id)
            .await?
            .map(|m| m.role)
    } else {
        None
    };

    Ok(Some(AuthContext {
        user,
        session_id,
        team_id,
        team_role,
    }))
}

/// Build the `Set-Cookie` header value for a session.
///
/// - `HttpOnly`: not readable from JS — XSS cannot exfiltrate the session.
/// - `Secure` when a `base_domain` is configured (i.e. behind Caddy with TLS);
///   omitted in local HTTP dev so the cookie still works (audit L2).
/// - `SameSite=Lax` rather than `Strict` (audit L1): the OAuth flow returns
///   the user via a top-level cross-site redirect from the provider, and
///   `Strict` would drop the session cookie on that navigation. `Lax` still
///   blocks the cookie on cross-site sub-requests (forms, images, fetch); the
///   residual top-level-navigation CSRF surface is covered by the
///   `X-Icefall-Request` header check on all mutating requests.
pub fn session_cookie(session_id: &str, base_domain: Option<&str>) -> String {
    let secure = if base_domain.is_some() {
        "; Secure"
    } else {
        ""
    };
    format!("icefall_session={session_id}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800{secure}")
}

fn sha256_hex(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_and_verify() {
        let hash = hash_password("test_password_123").unwrap();
        assert!(verify_password("test_password_123", &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn extract_session_from_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer session123".parse().unwrap());
        assert_eq!(extract_session_id(&headers), Some("session123".to_string()));
    }

    #[test]
    fn extract_session_from_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            "other=val; icefall_session=abc123; another=x"
                .parse()
                .unwrap(),
        );
        assert_eq!(extract_session_id(&headers), Some("abc123".to_string()));
    }

    #[test]
    fn sha256_produces_hex() {
        let hash = sha256_hex("test");
        assert_eq!(hash.len(), 64);
    }
}
