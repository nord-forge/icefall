use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{delete, post};
use axum::{Json, Router};
use serde::Deserialize;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/2fa/setup", post(setup_totp))
        .route("/auth/2fa/verify", post(verify_totp))
        .route("/auth/2fa/validate", post(validate_totp))
        .route("/auth/2fa/backup-codes", post(regenerate_backup_codes))
        .route("/auth/2fa", delete(disable_totp))
}

#[derive(Deserialize)]
struct VerifyRequest {
    code: String,
}

#[derive(Deserialize)]
struct ValidateRequest {
    /// The user_id from the pending 2FA login flow
    user_id: String,
    /// Either a 6-digit TOTP code or an 8-char backup code
    code: String,
}

#[derive(Deserialize)]
struct BackupCodesRequest {
    code: String,
}

#[derive(Deserialize)]
struct DisableRequest {
    code: String,
}

/// POST /api/v1/auth/2fa/setup
/// Generates a new TOTP secret and returns a QR code SVG + base32 secret.
/// Does NOT enable 2FA yet — the user must confirm with /verify.
async fn setup_totp(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if user.totp_enabled {
        return Err(ApiError::BadRequest(
            "Two-factor authentication is already enabled".into(),
        ));
    }

    // Generate a random secret
    let secret = Secret::generate_secret();
    let secret_base32 = secret.to_encoded().to_string();

    let issuer = "Icefall";
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret
            .to_bytes()
            .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?,
        Some(issuer.to_string()),
        user.email.clone(),
    )
    .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?;

    let otpauth_url = totp.get_url();

    // Generate QR code SVG
    let qr_svg = generate_qr_svg(&otpauth_url)?;

    // Store the pending secret (encrypted) on the user record.
    // It is not activated until /verify confirms.
    state
        .db
        .update_user_totp_secret(&user.id, Some(&secret_base32))
        .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "secret": secret_base32,
            "qr_svg": qr_svg,
            "otpauth_url": otpauth_url,
        }
    })))
}

/// POST /api/v1/auth/2fa/verify
/// Accepts a TOTP code to confirm setup, enables 2FA, returns backup codes.
async fn verify_totp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<VerifyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if user.totp_enabled {
        return Err(ApiError::BadRequest(
            "Two-factor authentication is already enabled".into(),
        ));
    }

    let secret_base32 = user.totp_secret.as_deref().ok_or_else(|| {
        ApiError::BadRequest("No pending 2FA setup found. Call /auth/2fa/setup first.".into())
    })?;

    // Validate the code against the pending secret
    let totp = build_totp(secret_base32, &user.email)?;
    if !check_totp_code(&totp, &body.code) {
        return Err(ApiError::BadRequest("Invalid TOTP code".into()));
    }

    // Generate 10 backup codes
    let (plain_codes, hashed_codes_json) = generate_backup_codes()?;

    // Enable 2FA and store hashed backup codes
    state
        .db
        .enable_user_totp(&user.id, &hashed_codes_json)
        .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "totp_enabled": true,
            "backup_codes": plain_codes,
        },
        "warning": "Save these backup codes securely. They will not be shown again."
    })))
}

/// POST /api/v1/auth/2fa/validate
/// Validates a TOTP code or backup code during login. Creates a session on success.
async fn validate_totp(
    State(state): State<AppState>,
    Json(body): Json<ValidateRequest>,
) -> Result<axum::response::Response, ApiError> {
    let user = state
        .db
        .get_user_by_id(&body.user_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Invalid user".into()))?;

    if !user.totp_enabled {
        return Err(ApiError::BadRequest(
            "2FA is not enabled for this user".into(),
        ));
    }

    let secret_base32 = user
        .totp_secret
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("2FA configuration error".into()))?;

    let code = body.code.trim();

    // Try TOTP code first (6 digits)
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        let totp = build_totp(secret_base32, &user.email)?;
        if check_totp_code(&totp, code) {
            return create_2fa_session(&state, &user).await;
        }
        return Err(ApiError::BadRequest("Invalid 2FA code".into()));
    }

    // Try backup code (8 alphanumeric characters)
    if code.len() == 8 && code.chars().all(|c| c.is_ascii_alphanumeric()) {
        if try_use_backup_code(&state, &user, code).await? {
            return create_2fa_session(&state, &user).await;
        }
        return Err(ApiError::BadRequest(
            "Invalid or already used backup code".into(),
        ));
    }

    Err(ApiError::BadRequest("Invalid 2FA code format".into()))
}

/// POST /api/v1/auth/2fa/backup-codes
/// Regenerates backup codes. Requires a valid TOTP code.
async fn regenerate_backup_codes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BackupCodesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if !user.totp_enabled {
        return Err(ApiError::BadRequest("2FA is not enabled".into()));
    }

    let secret_base32 = user
        .totp_secret
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("2FA configuration error".into()))?;

    let totp = build_totp(secret_base32, &user.email)?;
    if !check_totp_code(&totp, &body.code) {
        return Err(ApiError::BadRequest("Invalid TOTP code".into()));
    }

    let (plain_codes, hashed_codes_json) = generate_backup_codes()?;

    state
        .db
        .update_user_backup_codes(&user.id, &hashed_codes_json)
        .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "backup_codes": plain_codes,
        },
        "warning": "Save these backup codes securely. They replace all previous codes."
    })))
}

/// DELETE /api/v1/auth/2fa
/// Disables 2FA. Requires a valid TOTP code or backup code.
async fn disable_totp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DisableRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if !user.totp_enabled {
        return Err(ApiError::BadRequest("2FA is not enabled".into()));
    }

    let secret_base32 = user
        .totp_secret
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("2FA configuration error".into()))?;

    let code = body.code.trim();
    let mut authorized = false;

    // Try TOTP code
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        let totp = build_totp(secret_base32, &user.email)?;
        if check_totp_code(&totp, code) {
            authorized = true;
        }
    }

    // Try backup code
    if !authorized
        && code.len() == 8
        && code.chars().all(|c| c.is_ascii_alphanumeric())
        && try_use_backup_code(&state, &user, code).await?
    {
        authorized = true;
    }

    if !authorized {
        return Err(ApiError::BadRequest("Invalid code".into()));
    }

    state.db.disable_user_totp(&user.id).await?;

    Ok(Json(serde_json::json!({
        "message": "Two-factor authentication has been disabled"
    })))
}

// --- Helper functions ---

fn build_totp(secret_base32: &str, account: &str) -> Result<TOTP, ApiError> {
    let secret = Secret::Encoded(secret_base32.to_string());
    let secret_bytes = secret
        .to_bytes()
        .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?;

    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("Icefall".to_string()),
        account.to_string(),
    )
    .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))
}

/// Check a TOTP code. The TOTP is configured with skew=1, so it accepts
/// the current time step and one adjacent step in either direction (+-30s).
fn check_totp_code(totp: &TOTP, code: &str) -> bool {
    // check_current uses the skew configured in build_totp (1 step = +-30s)
    totp.check_current(code).unwrap_or(false)
}

fn generate_qr_svg(data: &str) -> Result<String, ApiError> {
    use qrcode::render::svg;
    use qrcode::QrCode;

    let code = QrCode::new(data.as_bytes())
        .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?;

    let svg: String = code
        .render::<svg::Color<'_>>()
        .min_dimensions(200, 200)
        .max_dimensions(300, 300)
        .quiet_zone(true)
        .build();

    Ok(svg)
}

/// Generate 10 random 8-character alphanumeric backup codes.
/// Returns (plain_codes, hashed_codes_json).
/// hashed_codes_json is a JSON array of objects: [{"hash": "...", "used": false}, ...]
fn generate_backup_codes() -> Result<(Vec<String>, String), ApiError> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::Rng;

    let mut rng = rand::rng();
    let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // No 0, O, 1, I for readability

    let mut plain_codes = Vec::with_capacity(10);
    let mut hashed_entries = Vec::with_capacity(10);

    for _ in 0..10 {
        let code: String = (0..8)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let hash = Argon2::default()
            .hash_password(code.as_bytes(), &salt)
            .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?;

        plain_codes.push(code);
        hashed_entries.push(serde_json::json!({
            "hash": hash.to_string(),
            "used": false,
        }));
    }

    let hashed_json =
        serde_json::to_string(&hashed_entries).map_err(|e| ApiError::Internal(Box::new(e)))?;

    Ok((plain_codes, hashed_json))
}

/// Try to use a backup code. If valid and unused, marks it as used and returns true.
async fn try_use_backup_code(
    state: &AppState,
    user: &crate::db::models::User,
    code: &str,
) -> Result<bool, ApiError> {
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};

    let Some(backup_json) = user.totp_backup_codes.as_deref() else {
        return Ok(false);
    };

    let mut entries: Vec<serde_json::Value> =
        serde_json::from_str(backup_json).map_err(|e| ApiError::Internal(Box::new(e)))?;

    let code_upper = code.to_uppercase();

    for entry in entries.iter_mut() {
        let used = entry
            .get("used")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        if used {
            continue;
        }

        let Some(hash_str) = entry.get("hash").and_then(|v| v.as_str()) else {
            continue;
        };

        let Ok(parsed) = PasswordHash::new(hash_str) else {
            continue;
        };

        if Argon2::default()
            .verify_password(code_upper.as_bytes(), &parsed)
            .is_ok()
        {
            // Mark as used
            entry["used"] = serde_json::Value::Bool(true);
            let updated_json =
                serde_json::to_string(&entries).map_err(|e| ApiError::Internal(Box::new(e)))?;
            state
                .db
                .update_user_backup_codes(&user.id, &updated_json)
                .await?;
            return Ok(true);
        }
    }

    Ok(false)
}

/// Create a session after successful 2FA validation and return the response with cookie.
async fn create_2fa_session(
    state: &AppState,
    user: &crate::db::models::User,
) -> Result<axum::response::Response, ApiError> {
    use axum::http::HeaderValue;
    use axum::response::IntoResponse;

    let expires_at = (chrono::Utc::now() + chrono::Duration::days(7))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let session = state
        .db
        .create_session(&user.id, &expires_at)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let body = serde_json::json!({
        "data": {
            "user": { "id": user.id, "email": user.email, "role": user.role, "totp_enabled": user.totp_enabled },
            "session_id": session.id,
        }
    });

    let cookie = format!(
        "icefall_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        session.id
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&cookie).map_err(|e| ApiError::Internal(Box::new(e)))?,
    );
    Ok((headers, Json(body)).into_response())
}
