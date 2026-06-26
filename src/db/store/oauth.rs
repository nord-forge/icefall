use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait OAuthStore: Send + Sync {
    // OAuth Identities
    async fn create_oauth_identity(
        &self,
        user_id: &str,
        provider: &str,
        provider_user_id: &str,
        provider_email: Option<&str>,
    ) -> Result<OAuthIdentity, DbError>;
    async fn get_oauth_identity(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> Result<Option<OAuthIdentity>, DbError>;
    async fn list_oauth_identities_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<OAuthIdentity>, DbError>;
    async fn delete_oauth_identity(&self, id: &str) -> Result<(), DbError>;

    // OAuth Settings
    async fn get_oauth_settings(&self) -> Result<Option<OAuthSettings>, DbError>;
    async fn upsert_oauth_settings(&self, settings: &OAuthSettings) -> Result<(), DbError>;

    // Registration Settings
    async fn get_registration_settings(&self) -> Result<RegistrationSettings, DbError>;
    async fn upsert_registration_settings(
        &self,
        allow_registration: bool,
        allowed_domains: Option<&str>,
        default_role: &str,
    ) -> Result<RegistrationSettings, DbError>;
}
