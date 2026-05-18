use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewUser;

use super::helpers::{generate_random_hex, hash_password};

#[derive(Deserialize)]
pub(super) struct InviteRequest {
    email: String,
    role: Option<String>,
}

pub(super) async fn invite_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
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
pub(super) struct AcceptInviteRequest {
    token: String,
    password: String,
}

pub(super) async fn accept_invite(
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
