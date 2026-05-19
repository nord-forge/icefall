//! Per-request team context and team-scoped authorization. `TeamCtx` resolves the caller's
//! active team; `verify_team_access` gates handlers on team ownership and role.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_with_team;
use crate::api::AppState;
use crate::db::models::User;

/// Team roles, ordered by privilege. `viewer < member < admin < owner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TeamRole {
    Viewer,
    Member,
    Admin,
    Owner,
}

impl TeamRole {
    /// Parse the role string stored in `team_memberships.role`. An
    /// unrecognized value is treated as the least-privileged role.
    fn parse(s: &str) -> Self {
        match s {
            "owner" => TeamRole::Owner,
            "admin" => TeamRole::Admin,
            "member" => TeamRole::Member,
            _ => TeamRole::Viewer,
        }
    }
}

/// The authenticated caller plus their resolved active team — an axum extractor for
/// tenant-scoped handlers. Fails 401 if unauthenticated, 403 (defensive) if teamless.
pub struct TeamCtx {
    pub user: User,
    pub team_id: String,
    pub team_role: TeamRole,
}

impl TeamCtx {
    /// Return `Ok` only if the caller's team owns `resource_team_id` with at least `min_role`.
    /// Wrong-team returns **404** (never revealing existence); insufficient role returns **403**.
    pub fn verify_team_access(
        &self,
        resource_team_id: &str,
        min_role: TeamRole,
    ) -> Result<(), ApiError> {
        if self.team_id != resource_team_id {
            return Err(ApiError::NotFound("resource not found".into()));
        }
        if self.team_role < min_role {
            return Err(ApiError::Forbidden(
                "Your team role does not permit this action".into(),
            ));
        }
        Ok(())
    }
}

impl FromRequestParts<AppState> for TeamCtx {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let ctx = authenticate_with_team(state, &parts.headers)
            .await?
            .ok_or_else(|| ApiError::Forbidden("Authentication required".into()))?;

        // Under always-a-team this is always Some; treat None defensively.
        let team_id = ctx
            .team_id
            .ok_or_else(|| ApiError::Forbidden("No team context for this user".into()))?;
        let team_role = TeamRole::parse(ctx.team_role.as_deref().unwrap_or(""));

        Ok(TeamCtx {
            user: ctx.user,
            team_id,
            team_role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(team_id: &str, role: TeamRole) -> TeamCtx {
        TeamCtx {
            user: User {
                id: "u1".into(),
                email: "u@example.com".into(),
                password_hash: String::new(),
                role: "viewer".into(),
                totp_secret: None,
                totp_enabled: false,
                totp_backup_codes: None,
                created_at: String::new(),
                updated_at: String::new(),
            },
            team_id: team_id.into(),
            team_role: role,
        }
    }

    #[test]
    fn role_ordering() {
        assert!(TeamRole::Owner > TeamRole::Admin);
        assert!(TeamRole::Admin > TeamRole::Member);
        assert!(TeamRole::Member > TeamRole::Viewer);
        assert_eq!(TeamRole::parse("owner"), TeamRole::Owner);
        assert_eq!(TeamRole::parse("nonsense"), TeamRole::Viewer);
    }

    #[test]
    fn cross_team_access_is_404_not_403() {
        // Must not leak that the resource exists in another team.
        let c = ctx("team-a", TeamRole::Owner);
        let err = c
            .verify_team_access("team-b", TeamRole::Viewer)
            .unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[test]
    fn insufficient_role_in_own_team_is_403() {
        let c = ctx("team-a", TeamRole::Viewer);
        let err = c
            .verify_team_access("team-a", TeamRole::Member)
            .unwrap_err();
        assert!(matches!(err, ApiError::Forbidden(_)));
    }

    #[test]
    fn sufficient_role_in_own_team_is_ok() {
        let c = ctx("team-a", TeamRole::Admin);
        assert!(c.verify_team_access("team-a", TeamRole::Member).is_ok());
        assert!(c.verify_team_access("team-a", TeamRole::Admin).is_ok());
    }
}
