use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait ProxyStore: Send + Sync {
    // Reverse proxy management (IF-149)
    async fn record_proxy_config_history(&self, app_id: &str, config: &str) -> Result<(), DbError>;
    async fn list_proxy_config_history(
        &self,
        app_id: &str,
    ) -> Result<Vec<ProxyConfigHistory>, DbError>;
    async fn latest_proxy_config_history(
        &self,
        app_id: &str,
    ) -> Result<Option<ProxyConfigHistory>, DbError>;
    async fn set_proxy_presets(&self, app_id: &str, presets: &str) -> Result<(), DbError>;
    async fn set_custom_proxy_config(&self, app_id: &str, config: &str) -> Result<(), DbError>;
    async fn clear_custom_proxy_config(&self, app_id: &str) -> Result<(), DbError>;
    async fn get_proxy_settings(&self) -> Result<ProxySettings, DbError>;
    async fn update_proxy_settings(
        &self,
        update: &UpdateProxySettings,
    ) -> Result<ProxySettings, DbError>;
}
