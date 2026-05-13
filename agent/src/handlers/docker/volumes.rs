use bollard::models::VolumeCreateRequest;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use crate::context::HandlerContext;
use crate::handlers::HandlerError;

#[derive(Debug, Deserialize)]
struct VolumeNameParams {
    name: String,
}

pub async fn volume_create(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: VolumeNameParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let config = VolumeCreateRequest {
        name: Some(p.name.clone()),
        ..Default::default()
    };
    ctx.docker.create_volume(config).await?;

    info!(name = %p.name, "volume created");
    Ok(serde_json::json!({ "ok": true }))
}

pub async fn volume_remove(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: VolumeNameParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    ctx.docker
        .remove_volume(
            &p.name,
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await?;

    info!(name = %p.name, "volume removed");
    Ok(serde_json::json!({ "ok": true }))
}

pub async fn volume_list(ctx: &HandlerContext, _params: Value) -> Result<Value, HandlerError> {
    let volumes = ctx
        .docker
        .list_volumes(None::<bollard::query_parameters::ListVolumesOptions>)
        .await?;

    let result: Vec<Value> = volumes
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            serde_json::json!({
                "name": v.name,
                "driver": v.driver,
                "mountpoint": v.mountpoint,
            })
        })
        .collect();

    Ok(serde_json::json!({ "volumes": result }))
}
