use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::{NewTeam, UpdateTeam};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/teams", get(list_teams).post(create_team))
        .route(
            "/teams/{id}",
            get(get_team).put(update_team).delete(delete_team),
        )
        .route("/teams/{id}/switch", post(switch_team))
        .route("/teams/{id}/members", get(list_members))
        .route(
            "/teams/{id}/members/{user_id}",
            put(update_member_role).delete(remove_member),
        )
        .route("/teams/{id}/invite", post(invite_member))
        .route("/teams/{id}/invitations", get(list_invitations))
        .route("/invitations/{token}/accept", post(accept_invitation))
        .route("/invitations/{token}", delete(decline_invitation))
        .route(
            "/servers/{server_id}/shares",
            get(list_server_shares).post(share_server),
        )
        .route(
            "/servers/{server_id}/shares/{team_id}",
            delete(revoke_share),
        )
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

async fn list_teams(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let teams = state.db.list_teams_for_user(&user.id).await?;
    Ok(Json(serde_json::json!({ "data": teams })))
}

#[derive(Deserialize)]
struct CreateTeamRequest {
    name: String,
}

async fn create_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateTeamRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Team name must not be empty".into()));
    }

    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(ApiError::BadRequest(
            "Team name must contain alphanumeric characters".into(),
        ));
    }

    if state.db.get_team_by_slug(&slug).await?.is_some() {
        return Err(ApiError::BadRequest(format!(
            "Team slug '{slug}' is already taken"
        )));
    }

    let team = state
        .db
        .create_team(&NewTeam {
            name,
            slug,
            owner_id: user.id,
        })
        .await?;

    Ok(Json(serde_json::json!({ "data": team })))
}

async fn get_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let team = state
        .db
        .get_team(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    let membership = state.db.get_team_membership(&id, &user.id).await?;
    if membership.is_none() {
        return Err(ApiError::NotFound("Team not found".into()));
    }

    let members = state.db.list_team_members(&id).await?;
    let resource_count = state.db.count_team_resources(&id).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "team": team,
            "members": members,
            "resource_count": resource_count,
        }
    })))
}

#[derive(Deserialize)]
struct UpdateTeamRequest {
    name: Option<String>,
    settings: Option<serde_json::Value>,
}

async fn update_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateTeamRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let membership = state
        .db
        .get_team_membership(&id, &user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if membership.role != "owner" && membership.role != "admin" {
        return Err(ApiError::BadRequest(
            "Only team owners and admins can update the team".into(),
        ));
    }

    let slug = body.name.as_ref().map(|n| slugify(n));

    let team = state
        .db
        .update_team(
            &id,
            &UpdateTeam {
                name: body.name,
                slug,
                settings: body.settings.map(|s| Some(s.to_string())),
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": team })))
}

async fn delete_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let team = state
        .db
        .get_team(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if team.owner_id != user.id {
        return Err(ApiError::BadRequest(
            "Only the team owner can delete the team".into(),
        ));
    }

    let resource_count = state.db.count_team_resources(&id).await?;
    if resource_count > 0 {
        return Err(ApiError::BadRequest(format!(
            "Team still has {resource_count} resources. Remove all apps, projects, and databases first."
        )));
    }

    state.db.delete_team(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn switch_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let membership = state
        .db
        .get_team_membership(&id, &user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    let session_id = crate::api::routes::auth::extract_session_id(&headers)
        .ok_or_else(|| ApiError::BadRequest("No session found".into()))?;

    state.db.set_session_team(&session_id, &id).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "team_id": id,
            "role": membership.role,
        },
        "message": "Switched team",
    })))
}

// --- Membership endpoints ---

async fn list_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let membership = state.db.get_team_membership(&id, &user.id).await?;
    if membership.is_none() {
        return Err(ApiError::NotFound("Team not found".into()));
    }

    let members = state.db.list_team_members(&id).await?;
    Ok(Json(serde_json::json!({ "data": members })))
}

#[derive(Deserialize)]
struct UpdateMemberRoleRequest {
    role: String,
}

async fn update_member_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((team_id, target_user_id)): Path<(String, String)>,
    Json(body): Json<UpdateMemberRoleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let membership = state
        .db
        .get_team_membership(&team_id, &user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if membership.role != "owner" && membership.role != "admin" {
        return Err(ApiError::BadRequest(
            "Only owners and admins can change roles".into(),
        ));
    }

    let valid_roles = ["admin", "member", "viewer"];
    if !valid_roles.contains(&body.role.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid role '{}'. Must be one of: admin, member, viewer",
            body.role
        )));
    }

    let team = state
        .db
        .get_team(&team_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if target_user_id == team.owner_id {
        return Err(ApiError::BadRequest(
            "Cannot change the owner's role".into(),
        ));
    }

    state
        .db
        .update_team_member_role(&team_id, &target_user_id, &body.role)
        .await?;

    Ok(Json(serde_json::json!({ "message": "Role updated" })))
}

async fn remove_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((team_id, target_user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let membership = state
        .db
        .get_team_membership(&team_id, &user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if membership.role != "owner" && membership.role != "admin" {
        return Err(ApiError::BadRequest(
            "Only owners and admins can remove members".into(),
        ));
    }

    let team = state
        .db
        .get_team(&team_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if target_user_id == team.owner_id {
        return Err(ApiError::BadRequest("Cannot remove the team owner".into()));
    }

    state
        .db
        .remove_team_member(&team_id, &target_user_id)
        .await?;
    Ok(Json(serde_json::json!({ "message": "Member removed" })))
}

// --- Invitation endpoints ---

#[derive(Deserialize)]
struct InviteRequest {
    email: String,
    role: String,
}

async fn invite_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
    Json(body): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let membership = state
        .db
        .get_team_membership(&team_id, &user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if membership.role != "owner" && membership.role != "admin" {
        return Err(ApiError::BadRequest(
            "Only owners and admins can invite members".into(),
        ));
    }

    let valid_roles = ["admin", "member", "viewer"];
    if !valid_roles.contains(&body.role.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid role '{}'. Must be one of: admin, member, viewer",
            body.role
        )));
    }

    let token = crate::db::models::new_id();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let invitation = state
        .db
        .create_team_invitation(
            &team_id,
            &body.email,
            &body.role,
            &token,
            &user.id,
            &expires_at,
        )
        .await?;

    Ok(Json(serde_json::json!({ "data": invitation })))
}

async fn list_invitations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let membership = state
        .db
        .get_team_membership(&team_id, &user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    if membership.role != "owner" && membership.role != "admin" {
        return Err(ApiError::BadRequest(
            "Only owners and admins can view invitations".into(),
        ));
    }

    let invitations = state.db.list_team_invitations(&team_id).await?;
    Ok(Json(serde_json::json!({ "data": invitations })))
}

async fn accept_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let invitation = state
        .db
        .get_team_invitation_by_token(&token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Invitation not found".into()))?;

    if invitation.expires_at < crate::db::models::now_iso8601() {
        state.db.delete_team_invitation(&invitation.id).await?;
        return Err(ApiError::BadRequest("Invitation expired".into()));
    }

    if invitation.email.to_lowercase() != user.email.to_lowercase() {
        return Err(ApiError::BadRequest(
            "This invitation was sent to a different email address".into(),
        ));
    }

    let existing = state
        .db
        .get_team_membership(&invitation.team_id, &user.id)
        .await?;
    if existing.is_some() {
        state.db.delete_team_invitation(&invitation.id).await?;
        return Err(ApiError::BadRequest(
            "You are already a member of this team".into(),
        ));
    }

    state
        .db
        .add_team_member(
            &invitation.team_id,
            &user.id,
            &invitation.role,
            Some(&invitation.invited_by),
        )
        .await?;

    state.db.delete_team_invitation(&invitation.id).await?;

    let team = state.db.get_team(&invitation.team_id).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "team": team,
            "role": invitation.role,
        },
        "message": "Invitation accepted",
    })))
}

async fn decline_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let invitation = state
        .db
        .get_team_invitation_by_token(&token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Invitation not found".into()))?;

    state.db.delete_team_invitation(&invitation.id).await?;

    Ok(Json(
        serde_json::json!({ "message": "Invitation declined" }),
    ))
}

// --- Server sharing endpoints ---

async fn list_server_shares(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let shares = state.db.list_server_shares(&server_id).await?;
    Ok(Json(serde_json::json!({ "data": shares })))
}

#[derive(Deserialize)]
struct ShareServerRequest {
    team_id: String,
    access_level: String,
}

async fn share_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    Json(body): Json<ShareServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let valid_levels = ["deploy", "read-only"];
    if !valid_levels.contains(&body.access_level.as_str()) {
        return Err(ApiError::BadRequest(
            "access_level must be 'deploy' or 'read-only'".into(),
        ));
    }

    let share = state
        .db
        .share_server_with_team(&server_id, &body.team_id, &body.access_level, &user.id)
        .await?;

    Ok(Json(serde_json::json!({ "data": share })))
}

async fn revoke_share(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, team_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    state.db.revoke_server_share(&server_id, &team_id).await?;
    Ok(Json(serde_json::json!({ "message": "Share revoked" })))
}
