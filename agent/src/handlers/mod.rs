pub mod build;
pub mod caddy;
pub mod docker;
pub mod health;
pub mod logs;
pub mod metrics;
pub mod system;
pub mod terminal;

use icefall_common::protocol::AgentMessage;
use serde_json::Value;

use crate::context::HandlerContext;

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("unknown method: {0}")]
    UnknownMethod(String),
    #[error("invalid params: {0}")]
    InvalidParams(String),
    #[error("docker: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub async fn dispatch(ctx: &HandlerContext, id: String, method: &str, params: Value) -> AgentMessage {
    let result = match method {
        "container.create" => docker::container_create(ctx, params).await,
        "container.start" => docker::container_start(ctx, params).await,
        "container.stop" => docker::container_stop(ctx, params).await,
        "container.remove" => docker::container_remove(ctx, params).await,
        "container.list" => docker::container_list(ctx, params).await,
        "container.inspect" => docker::container_inspect(ctx, params).await,

        "image.pull" => docker::image_pull(ctx, params).await,
        "image.build" => docker::image_build(ctx, params).await,

        "volume.create" => docker::volume_create(ctx, params).await,
        "volume.remove" => docker::volume_remove(ctx, params).await,
        "volume.list" => docker::volume_list(ctx, params).await,

        "network.create" => docker::network_create(ctx, params).await,
        "network.remove" => docker::network_remove(ctx, params).await,

        "build.run" => build::run_build(ctx, params).await,

        "container.logs.subscribe" => logs::subscribe(ctx, params).await,
        "container.logs.unsubscribe" => logs::unsubscribe(ctx, params).await,

        "health.check" => health::check(ctx, params).await,

        "terminal.open" => terminal::open(ctx, params).await,
        "terminal.input" => terminal::input(ctx, params).await,
        "terminal.resize" => terminal::resize(ctx, params).await,
        "terminal.close" => terminal::close(ctx, params).await,

        "caddy.add_route" => caddy::add_route(ctx, params).await,
        "caddy.update_route" => caddy::update_route(ctx, params).await,
        "caddy.remove_route" => caddy::remove_route(ctx, params).await,

        "system.update" => system::update(ctx, params).await,

        _ => Err(HandlerError::UnknownMethod(method.to_string())),
    };

    match result {
        Ok(value) => AgentMessage::Response {
            id,
            result: Some(value),
            error: None,
        },
        Err(e) => AgentMessage::Response {
            id,
            result: None,
            error: Some(e.to_string()),
        },
    }
}
