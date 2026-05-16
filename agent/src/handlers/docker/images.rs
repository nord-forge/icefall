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

fn parse_transfer_id(hex: &str) -> Result<[u8; 16], HandlerError> {
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| HandlerError::InvalidParams("invalid transfer_id hex".to_string()))?;
    bytes
        .try_into()
        .map_err(|_| HandlerError::InvalidParams("transfer_id must be 16 bytes".to_string()))
}

#[derive(Debug, Deserialize)]
struct LoadBeginParams {
    /// 32-char hex string identifying this transfer.
    transfer_id: String,
    total_chunks: u32,
    chunk_size: usize,
    /// Hex SHA-256 of the fully reassembled (compressed) tar archive.
    total_sha256: String,
}

/// Begin an image transfer: the agent allocates a temp file to reassemble
/// incoming binary chunk frames into.
pub async fn image_load_begin(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: LoadBeginParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;
    let transfer_id = parse_transfer_id(&p.transfer_id)?;

    ctx.transfers
        .begin(transfer_id, p.total_chunks, p.total_sha256, p.chunk_size)
        .await
        .map_err(HandlerError::Other)?;

    info!(transfer_id = %p.transfer_id, total_chunks = p.total_chunks, "image transfer begun");
    Ok(serde_json::json!({ "ok": true }))
}

#[derive(Debug, Deserialize)]
struct LoadCommitParams {
    transfer_id: String,
}

/// Commit an image transfer: verify all chunks arrived and the whole-file
/// checksum matches, then load the reassembled archive into Docker.
pub async fn image_load_commit(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: LoadCommitParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;
    let transfer_id = parse_transfer_id(&p.transfer_id)?;

    let path = ctx
        .transfers
        .commit(transfer_id)
        .await
        .map_err(HandlerError::Other)?;

    // Decompress (gzip) and load into Docker.
    let result = load_image_from_gzip(ctx, &path).await;
    ctx.transfers.cleanup(transfer_id).await;
    result?;

    info!(transfer_id = %p.transfer_id, "image transfer committed and loaded");
    Ok(serde_json::json!({ "ok": true }))
}

async fn load_image_from_gzip(
    ctx: &HandlerContext,
    path: &std::path::Path,
) -> Result<(), HandlerError> {
    use std::io::Read;

    let compressed =
        std::fs::read(path).map_err(|e| HandlerError::Other(format!("read archive: {e}")))?;
    let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
    let mut tar_bytes = Vec::new();
    decoder
        .read_to_end(&mut tar_bytes)
        .map_err(|e| HandlerError::Other(format!("gunzip: {e}")))?;

    let mut stream = ctx.docker.import_image(
        bollard::query_parameters::ImportImageOptions::default(),
        bollard::body_full(tar_bytes.into()),
        None,
    );
    while let Some(result) = stream.next().await {
        result?;
    }
    Ok(())
}
