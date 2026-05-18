use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::OAuthSettings;

/// GET /api/v1/settings/oauth/providers
/// Public endpoint: returns which OAuth providers are enabled (no secrets).
pub(super) async fn get_enabled_providers(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let settings = state.db.get_oauth_settings().await?;

    let (github, google) = match settings {
        Some(ref s) => (
            s.github_enabled && s.github_client_id.as_ref().is_some_and(|id| !id.is_empty()),
            s.google_enabled && s.google_client_id.as_ref().is_some_and(|id| !id.is_empty()),
        ),
        None => (false, false),
    };

    Ok(Json(serde_json::json!({
        "data": {
            "github": github,
            "google": google,
        }
    })))
}

/// DELETE /api/v1/auth/oauth/{provider}/unlink
/// Unlinks an OAuth provider from the authenticated user.
///
/// When the user has other OAuth providers linked, unlinking is allowed freely.
/// When this is the last provider, the request body must include
/// `{ "password": "<current>" }` to prove the user can still log in with a password.
pub(super) async fn oauth_unlink(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Option<Json<UnlinkBody>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let identities = state.db.list_oauth_identities_for_user(&user.id).await?;

    let identity = identities
        .iter()
        .find(|i| i.provider == provider)
        .ok_or_else(|| {
            ApiError::NotFound(format!("No {provider} identity linked to your account"))
        })?;

    let other_providers = identities.iter().filter(|i| i.provider != provider).count();

    if other_providers == 0 {
        // Last OAuth provider: require password confirmation to prove the
        // user has another way to log in.
        let password = body.and_then(|b| b.password.clone()).ok_or_else(|| {
            ApiError::BadRequest(
                "This is your only linked provider. Supply your password to confirm \
                     you can still log in: { \"password\": \"...\" }"
                    .into(),
            )
        })?;

        if !verify_password(&password, &user.password_hash) {
            return Err(ApiError::BadRequest("Incorrect password".into()));
        }
    }

    state.db.delete_oauth_identity(&identity.id).await?;

    Ok(Json(serde_json::json!({
        "message": format!("{provider} has been unlinked from your account")
    })))
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

#[derive(Deserialize)]
pub(super) struct UnlinkBody {
    password: Option<String>,
}

/// GET /api/v1/auth/oauth/identities
/// Lists OAuth identities for the authenticated user.
pub(super) async fn list_oauth_identities(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let identities = state.db.list_oauth_identities_for_user(&user.id).await?;

    let data: Vec<serde_json::Value> = identities
        .into_iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id,
                "provider": i.provider,
                "provider_email": i.provider_email,
                "created_at": i.created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": data })))
}

/// GET /api/v1/settings/oauth
/// Returns OAuth settings for admin configuration.
pub(super) async fn get_oauth_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if user.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    let settings = state.db.get_oauth_settings().await?;

    let base = state.config.base_domain.as_deref().map_or_else(
        || "http://localhost:3000".to_string(),
        |d| format!("https://{d}"),
    );

    match settings {
        Some(s) => Ok(Json(serde_json::json!({
            "data": {
                "github_client_id": s.github_client_id,
                "github_has_secret": s.github_client_secret.is_some(),
                "github_enabled": s.github_enabled,
                "github_callback_url": format!("{base}/api/v1/auth/oauth/github/callback"),
                "google_client_id": s.google_client_id,
                "google_has_secret": s.google_client_secret.is_some(),
                "google_enabled": s.google_enabled,
                "google_callback_url": format!("{base}/api/v1/auth/oauth/google/callback"),
            }
        }))),
        None => Ok(Json(serde_json::json!({
            "data": {
                "github_client_id": null,
                "github_has_secret": false,
                "github_enabled": false,
                "github_callback_url": format!("{base}/api/v1/auth/oauth/github/callback"),
                "google_client_id": null,
                "google_has_secret": false,
                "google_enabled": false,
                "google_callback_url": format!("{base}/api/v1/auth/oauth/google/callback"),
            }
        }))),
    }
}

#[derive(Deserialize)]
pub(super) struct UpdateOAuthSettingsRequest {
    github_client_id: Option<String>,
    github_client_secret: Option<String>,
    github_enabled: Option<bool>,
    google_client_id: Option<String>,
    google_client_secret: Option<String>,
    google_enabled: Option<bool>,
}

/// PUT /api/v1/settings/oauth
/// Updates OAuth settings (admin only).
pub(super) async fn update_oauth_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateOAuthSettingsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    if user.role != "admin" {
        return Err(ApiError::BadRequest("Admin access required".into()));
    }

    // Merge with existing settings
    let existing = state
        .db
        .get_oauth_settings()
        .await?
        .unwrap_or(OAuthSettings {
            github_client_id: None,
            github_client_secret: None,
            github_enabled: false,
            google_client_id: None,
            google_client_secret: None,
            google_enabled: false,
        });

    let updated = OAuthSettings {
        github_client_id: body.github_client_id.or(existing.github_client_id),
        github_client_secret: body.github_client_secret.or(existing.github_client_secret),
        github_enabled: body.github_enabled.unwrap_or(existing.github_enabled),
        google_client_id: body.google_client_id.or(existing.google_client_id),
        google_client_secret: body.google_client_secret.or(existing.google_client_secret),
        google_enabled: body.google_enabled.unwrap_or(existing.google_enabled),
    };

    state.db.upsert_oauth_settings(&updated).await?;

    let base = state.config.base_domain.as_deref().map_or_else(
        || "http://localhost:3000".to_string(),
        |d| format!("https://{d}"),
    );

    Ok(Json(serde_json::json!({
        "data": {
            "github_client_id": updated.github_client_id,
            "github_has_secret": updated.github_client_secret.is_some(),
            "github_enabled": updated.github_enabled,
            "github_callback_url": format!("{base}/api/v1/auth/oauth/github/callback"),
            "google_client_id": updated.google_client_id,
            "google_has_secret": updated.google_client_secret.is_some(),
            "google_enabled": updated.google_enabled,
            "google_callback_url": format!("{base}/api/v1/auth/oauth/google/callback"),
        },
        "message": "OAuth settings updated"
    })))
}
