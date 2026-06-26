use async_trait::async_trait;

use crate::db::models::*;
use crate::db::DbError;

#[async_trait]
pub trait NetworkingStore: Send + Sync {
    // Domains
    async fn add_domain(&self, domain: &NewDomain) -> Result<Domain, DbError>;
    async fn list_domains(&self, app_id: &str) -> Result<Vec<Domain>, DbError>;
    async fn update_domain_status(
        &self,
        id: &str,
        verified: bool,
        ssl_status: &str,
    ) -> Result<(), DbError>;
    async fn delete_domain(&self, id: &str) -> Result<(), DbError>;
    /// Set one domain as the app's primary, clearing the flag on the rest.
    async fn set_primary_domain(&self, app_id: &str, domain_id: &str) -> Result<(), DbError>;
    async fn list_all_domains(&self) -> Result<Vec<Domain>, DbError>;
    async fn update_domain_ssl_info(
        &self,
        id: &str,
        issuer: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<(), DbError>;

    // Webhook endpoints
    async fn list_webhook_endpoints(&self) -> Result<Vec<WebhookEndpoint>, DbError>;
    async fn create_webhook_endpoint(
        &self,
        endpoint: &NewWebhookEndpoint,
    ) -> Result<WebhookEndpoint, DbError>;
    async fn delete_webhook_endpoint(&self, id: &str) -> Result<(), DbError>;
    async fn create_webhook_delivery(
        &self,
        endpoint_id: &str,
        event: &str,
        status_code: Option<i32>,
        response_time_ms: Option<i32>,
        attempt: i32,
        error: Option<&str>,
    ) -> Result<(), DbError>;
    async fn list_webhook_deliveries(
        &self,
        endpoint_id: &str,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, DbError>;

    // Public ports
    async fn allocate_public_port(
        &self,
        resource_type: &str,
        resource_id: &str,
        port: i32,
        ip_whitelist: Option<&str>,
    ) -> Result<PublicPort, DbError>;
    /// Allocate the lowest free port in the inclusive range for a resource,
    /// retrying past races. Errors if the range is exhausted.
    async fn allocate_free_public_port(
        &self,
        resource_type: &str,
        resource_id: &str,
        range_start: i32,
        range_end: i32,
        ip_whitelist: Option<&str>,
    ) -> Result<PublicPort, DbError>;
    async fn release_public_port(&self, resource_id: &str) -> Result<(), DbError>;
    async fn get_public_port(&self, resource_id: &str) -> Result<Option<PublicPort>, DbError>;
}
