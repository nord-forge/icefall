use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait UserStore: Send + Sync {
    // Users
    async fn create_user(&self, user: &NewUser) -> Result<User, DbError>;
    /// Atomically create the first admin account, with its personal team;
    /// fails with `DbError::Duplicate` if any user already exists (audit H8).
    async fn create_first_admin(&self, user: &NewUser) -> Result<User, DbError>;
    /// Create a user together with their personal team, atomically. The
    /// standard user-creation path under the always-a-team tenancy model.
    async fn create_user_with_personal_team(&self, user: &NewUser)
        -> Result<(User, Team), DbError>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, DbError>;
    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, DbError>;
    async fn list_users(&self) -> Result<Vec<User>, DbError>;

    // TOTP / 2FA
    async fn update_user_totp_secret(
        &self,
        user_id: &str,
        secret: Option<&str>,
    ) -> Result<(), DbError>;
    async fn enable_user_totp(&self, user_id: &str, backup_codes: &str) -> Result<(), DbError>;
    async fn disable_user_totp(&self, user_id: &str) -> Result<(), DbError>;
    async fn update_user_backup_codes(
        &self,
        user_id: &str,
        backup_codes: &str,
    ) -> Result<(), DbError>;

    // User profile updates
    async fn update_user_password(&self, user_id: &str, password_hash: &str)
        -> Result<(), DbError>;
    async fn update_user_email(&self, user_id: &str, email: &str) -> Result<(), DbError>;

    // User deletion
    async fn delete_user(&self, user_id: &str) -> Result<(), DbError>;
    async fn count_admin_users(&self) -> Result<i64, DbError>;

    // User preferences
    async fn get_user_preferences(&self, user_id: &str) -> Result<serde_json::Value, DbError>;
    async fn update_user_preferences(
        &self,
        user_id: &str,
        preferences: &serde_json::Value,
    ) -> Result<(), DbError>;

    // Admin 2FA reset
    async fn admin_reset_user_2fa(&self, user_id: &str) -> Result<(), DbError>;

    // Sessions
    async fn create_session(&self, user_id: &str, expires_at: &str) -> Result<Session, DbError>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, DbError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), DbError>;
    async fn delete_user_sessions(&self, user_id: &str) -> Result<(), DbError>;
    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, DbError>;
    async fn delete_user_sessions_except(
        &self,
        user_id: &str,
        keep_session_id: &str,
    ) -> Result<(), DbError>;

    // API Tokens
    async fn create_api_token(
        &self,
        user_id: &str,
        name: &str,
        token_hash: &str,
        expires_at: Option<&str>,
        team_id: Option<&str>,
        abilities: Option<&str>,
    ) -> Result<ApiToken, DbError>;
    async fn get_api_token_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, DbError>;
    async fn list_api_tokens(&self, user_id: &str) -> Result<Vec<ApiToken>, DbError>;
    async fn delete_api_token(&self, id: &str) -> Result<(), DbError>;
    async fn update_token_last_used(&self, id: &str) -> Result<(), DbError>;

    // Invitations
    async fn create_invitation(
        &self,
        email: &str,
        role: &str,
        token: &str,
        expires_at: &str,
    ) -> Result<Invitation, DbError>;
    async fn get_invitation_by_token(&self, token: &str) -> Result<Option<Invitation>, DbError>;
    async fn delete_invitation(&self, id: &str) -> Result<(), DbError>;

    // Onboarding
    async fn get_onboarding(
        &self,
    ) -> Result<Option<(String, String, String, Option<String>)>, DbError>;
    async fn create_onboarding(&self, started_at: &str) -> Result<(), DbError>;
    async fn update_onboarding_state(
        &self,
        current_step: &str,
        completed_steps: &str,
        completed_at: Option<&str>,
    ) -> Result<(), DbError>;
}
