use std::collections::HashMap;

use bollard::models::{
    ContainerCreateBody, HostConfig, NetworkConnectRequest, NetworkCreateRequest, PortBinding,
    VolumeCreateRequest,
};
use bollard::query_parameters::{
    BuildImageOptions, CreateContainerOptions, CreateImageOptions, ListContainersOptions,
    RemoveContainerOptions, StopContainerOptions,
};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use super::HandlerError;
use crate::context::HandlerContext;

// --- Container operations ---

#[derive(Debug, Deserialize)]
struct CreateContainerParams {
    name: String,
    image: String,
    #[serde(default)]
    env: Vec<String>,
    #[serde(default)]
    cmd: Option<Vec<String>>,
    #[serde(default)]
    labels: HashMap<String, String>,
    #[serde(default)]
    ports: Vec<PortMapping>,
    #[serde(default)]
    volumes: Vec<VolumeMount>,
    network: Option<String>,
    restart_policy: Option<String>,
    memory_bytes: Option<i64>,
    cpu_shares: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PortMapping {
    host_port: Option<u16>,
    container_port: u16,
    #[serde(default = "default_protocol")]
    protocol: String,
}

fn default_protocol() -> String {
    "tcp".to_string()
}

#[derive(Debug, Deserialize)]
struct VolumeMount {
    source: String,
    target: String,
    #[serde(default)]
    read_only: bool,
}

#[derive(Debug, Deserialize)]
struct ContainerIdParams {
    id: String,
}

#[derive(Debug, Deserialize)]
struct StopContainerParams {
    id: String,
    timeout_secs: Option<i64>,
}

pub async fn container_create(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: CreateContainerParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let mut port_bindings = HashMap::new();
    let mut exposed_ports = Vec::new();

    for port in &p.ports {
        let key = format!("{}/{}", port.container_port, port.protocol);
        exposed_ports.push(key.clone());
        port_bindings.insert(
            key,
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: port.host_port.map(|hp| hp.to_string()),
            }]),
        );
    }

    let binds: Vec<String> = p
        .volumes
        .iter()
        .map(|v| {
            if v.read_only {
                format!("{}:{}:ro", v.source, v.target)
            } else {
                format!("{}:{}", v.source, v.target)
            }
        })
        .collect();

    let restart_policy = p
        .restart_policy
        .as_deref()
        .map(|policy| bollard::models::RestartPolicy {
            name: Some(match policy {
                "always" => bollard::models::RestartPolicyNameEnum::ALWAYS,
                "unless-stopped" => bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED,
                "on-failure" => bollard::models::RestartPolicyNameEnum::ON_FAILURE,
                _ => bollard::models::RestartPolicyNameEnum::NO,
            }),
            maximum_retry_count: None,
        });

    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        binds: Some(binds),
        memory: p.memory_bytes,
        cpu_shares: p.cpu_shares,
        restart_policy,
        ..Default::default()
    };

    let config = ContainerCreateBody {
        image: Some(p.image.clone()),
        env: Some(p.env.clone()),
        cmd: p.cmd.clone(),
        exposed_ports: Some(exposed_ports),
        host_config: Some(host_config),
        labels: Some(p.labels.clone()),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: Some(p.name.clone()),
        ..Default::default()
    };

    let response = ctx.docker.create_container(Some(options), config).await?;

    if let Some(ref network) = p.network {
        ctx.docker
            .connect_network(
                network,
                NetworkConnectRequest {
                    container: response.id.clone(),
                    ..Default::default()
                },
            )
            .await?;
    }

    info!(name = %p.name, id = %response.id, "container created");
    Ok(serde_json::json!({ "id": response.id }))
}

pub async fn container_start(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: ContainerIdParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    ctx.docker.start_container(&p.id, None).await?;

    info!(id = %p.id, "container started");
    Ok(serde_json::json!({ "ok": true }))
}

pub async fn container_stop(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: StopContainerParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let options = StopContainerOptions {
        t: Some(p.timeout_secs.unwrap_or(30) as i32),
        signal: None,
    };
    ctx.docker.stop_container(&p.id, Some(options)).await?;

    info!(id = %p.id, "container stopped");
    Ok(serde_json::json!({ "ok": true }))
}

pub async fn container_remove(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: ContainerIdParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let options = RemoveContainerOptions {
        force: true,
        v: true,
        ..Default::default()
    };
    ctx.docker.remove_container(&p.id, Some(options)).await?;

    info!(id = %p.id, "container removed");
    Ok(serde_json::json!({ "ok": true }))
}

pub async fn container_inspect(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: ContainerIdParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let info = ctx.docker.inspect_container(&p.id, None).await?;

    Ok(serde_json::json!({
        "id": info.id,
        "state": info.state,
        "network_settings": info.network_settings,
    }))
}

pub async fn container_list(ctx: &HandlerContext, _params: Value) -> Result<Value, HandlerError> {
    let options = ListContainersOptions {
        all: true,
        ..Default::default()
    };

    let containers = ctx.docker.list_containers(Some(options)).await?;

    let result: Vec<Value> = containers
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "names": c.names,
                "image": c.image,
                "state": c.state,
                "status": c.status,
                "labels": c.labels,
            })
        })
        .collect();

    Ok(serde_json::json!({ "containers": result }))
}

// --- Image operations ---

#[derive(Debug, Deserialize)]
struct PullImageParams {
    image: String,
    #[serde(default = "default_tag")]
    tag: String,
}

fn default_tag() -> String {
    "latest".to_string()
}

pub async fn image_pull(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: PullImageParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let options = CreateImageOptions {
        from_image: Some(p.image.clone()),
        tag: Some(p.tag.clone()),
        ..Default::default()
    };

    let mut stream = ctx.docker.create_image(Some(options), None, None);
    while let Some(result) = stream.next().await {
        result?;
    }

    info!(image = %p.image, tag = %p.tag, "image pulled");
    Ok(serde_json::json!({ "ok": true, "image": format!("{}:{}", p.image, p.tag) }))
}

#[derive(Debug, Deserialize)]
struct BuildImageParams {
    tag: String,
    context_tar_b64: String,
    #[serde(default)]
    dockerfile: Option<String>,
}

pub async fn image_build(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: BuildImageParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let context_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &p.context_tar_b64,
    )
    .map_err(|e| HandlerError::Other(format!("invalid base64 context: {e}")))?;

    let mut options = BuildImageOptions {
        t: Some(p.tag.clone()),
        rm: true,
        ..Default::default()
    };
    if let Some(ref df) = p.dockerfile {
        options.dockerfile = df.clone();
    }

    let mut stream = ctx.docker.build_image(
        options,
        None,
        Some(bollard::body_full(context_bytes.into())),
    );

    while let Some(result) = stream.next().await {
        let info = result?;
        if let Some(ref stream_text) = info.stream {
            let trimmed = stream_text.trim();
            if !trimmed.is_empty() {
                let _ = ctx
                    .event_tx
                    .send(icefall_common::protocol::AgentMessage::Event {
                        event_type: "build.output".to_string(),
                        data: serde_json::json!({ "line": trimmed }),
                    });
            }
        }
    }

    info!(tag = %p.tag, "image built");
    Ok(serde_json::json!({ "ok": true, "tag": p.tag }))
}

// --- Volume operations ---

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

// --- Network operations ---

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
