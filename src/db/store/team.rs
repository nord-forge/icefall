use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait TeamStore: Send + Sync {
    // Team-scoped queries
    async fn list_apps_by_team(&self, team_id: &str) -> Result<Vec<App>, DbError>;
    async fn list_projects_by_team(&self, team_id: &str) -> Result<Vec<Project>, DbError>;
    async fn list_managed_dbs_by_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<ManagedDatabase>, DbError>;
    async fn list_ssh_keys_by_team(&self, team_id: &str) -> Result<Vec<SshKey>, DbError>;
    async fn list_registries_by_team(&self, team_id: &str) -> Result<Vec<Registry>, DbError>;
    async fn list_notification_channels_by_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<Notification>, DbError>;
    async fn list_api_tokens_by_team(&self, team_id: &str) -> Result<Vec<ApiToken>, DbError>;
    /// Fetch one app only if it belongs to `team_id`; `None` otherwise
    /// (covers both "no such app" and "exists in another team").
    async fn get_app_for_team(&self, team_id: &str, app_id: &str) -> Result<Option<App>, DbError>;
    /// Fetch one managed database only if it belongs to `team_id`.
    async fn get_managed_db_for_team(
        &self,
        team_id: &str,
        db_id: &str,
    ) -> Result<Option<ManagedDatabase>, DbError>;

    // Set team_id on resources
    async fn set_app_team(&self, app_id: &str, team_id: &str) -> Result<(), DbError>;
    async fn set_project_team(&self, project_id: &str, team_id: &str) -> Result<(), DbError>;
    async fn set_database_team(&self, db_id: &str, team_id: &str) -> Result<(), DbError>;

    // Cross-team server sharing
    async fn share_server_with_team(
        &self,
        server_id: &str,
        team_id: &str,
        access_level: &str,
        granted_by: &str,
    ) -> Result<ServerTeamAccess, DbError>;
    async fn revoke_server_share(&self, server_id: &str, team_id: &str) -> Result<(), DbError>;
    async fn list_server_shares(&self, server_id: &str) -> Result<Vec<ServerTeamAccess>, DbError>;
    async fn list_servers_shared_with_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<(Server, String)>, DbError>;

    // Teams
    async fn create_team(&self, team: &NewTeam) -> Result<Team, DbError>;
    async fn get_team(&self, id: &str) -> Result<Option<Team>, DbError>;
    async fn get_team_by_slug(&self, slug: &str) -> Result<Option<Team>, DbError>;
    async fn list_teams_for_user(&self, user_id: &str) -> Result<Vec<Team>, DbError>;
    async fn update_team(&self, id: &str, update: &UpdateTeam) -> Result<Team, DbError>;
    async fn delete_team(&self, id: &str) -> Result<(), DbError>;
    async fn count_team_resources(&self, team_id: &str) -> Result<i64, DbError>;

    // Team memberships
    async fn add_team_member(
        &self,
        team_id: &str,
        user_id: &str,
        role: &str,
        invited_by: Option<&str>,
    ) -> Result<TeamMembership, DbError>;
    async fn list_team_members(&self, team_id: &str) -> Result<Vec<TeamMember>, DbError>;
    async fn get_team_membership(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<Option<TeamMembership>, DbError>;
    async fn update_team_member_role(
        &self,
        team_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), DbError>;
    async fn remove_team_member(&self, team_id: &str, user_id: &str) -> Result<(), DbError>;

    // Team invitations
    async fn create_team_invitation(
        &self,
        team_id: &str,
        email: &str,
        role: &str,
        token: &str,
        invited_by: &str,
        expires_at: &str,
    ) -> Result<TeamInvitation, DbError>;
    async fn get_team_invitation_by_token(
        &self,
        token: &str,
    ) -> Result<Option<TeamInvitation>, DbError>;
    async fn list_team_invitations(&self, team_id: &str) -> Result<Vec<TeamInvitation>, DbError>;
    async fn delete_team_invitation(&self, id: &str) -> Result<(), DbError>;

    // Session team context
    async fn set_session_team(&self, session_id: &str, team_id: &str) -> Result<(), DbError>;
}
