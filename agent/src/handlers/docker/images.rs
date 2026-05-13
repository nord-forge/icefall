use bollard::query_parameters::{BuildImageOptions, CreateImageOptions};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use crate::context::HandlerContext;
use crate::handlers::HandlerError;

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
