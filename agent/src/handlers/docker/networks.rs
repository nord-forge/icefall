use bollard::models::NetworkCreateRequest;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use crate::context::HandlerContext;
use crate::handlers::HandlerError;

#[derive(Debug, Deserialize)]
struct NetworkNameParams {
    name: String,
}

pub async fn network_create(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: NetworkNameParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let config = NetworkCreateRequest {
        name: p.name.clone(),
        driver: Some("bridge".to_string()),
        ..Default::default()
    };
    let response = ctx.docker.create_network(config).await?;

    info!(name = %p.name, "network created");
    Ok(serde_json::json!({ "id": response.id }))
}

pub async fn network_remove(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: NetworkNameParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    ctx.docker.remove_network(&p.name).await?;

    info!(name = %p.name, "network removed");
    Ok(serde_json::json!({ "ok": true }))
}
