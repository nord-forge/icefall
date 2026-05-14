use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookEndpoint {
    pub id: String,
    pub name: String,
    pub url: String,
    pub method: String,
    pub secret: Option<String>,
    pub headers: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewWebhookEndpoint {
    pub name: String,
    pub url: String,
    pub method: Option<String>,
    pub secret: Option<String>,
    pub headers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookDelivery {
    pub id: String,
    pub endpoint_id: String,
    pub event: String,
    pub status_code: Option<i32>,
    pub response_time_ms: Option<i32>,
    pub attempt: i32,
    pub error: Option<String>,
    pub delivered_at: String,
}
