use std::path::{Path, PathBuf};
use std::time::Instant;

use bollard::query_parameters::BuildImageOptions;
use bytes::Bytes;
use futures_util::StreamExt;
use icefall_common::build::detect::detect;
use icefall_common::build::dockerfile::{generate_dockerfile, generate_dockerignore};
use icefall_common::build::{BuildConfig, Framework};
use icefall_common::protocol::AgentMessage;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info};

use super::HandlerError;
use crate::context::HandlerContext;

#[derive(Debug, Deserialize)]
struct BuildRunParams {
    repo_url: String,
    branch: Option<String>,
    sha: Option<String>,
    token: Option<String>,
    deploy_id: String,
    app_name: String,
    #[serde(default)]
    env_vars: Vec<String>,
    config: Option<BuildConfig>,
}

pub async fn run_build(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: BuildRunParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let start = Instant::now();
    let build_dir = PathBuf::from(&ctx.config.data_dir)
        .join("builds")
        .join(&p.deploy_id);

    let result = execute_build(ctx, &p, &build_dir).await;

    let _ = tokio::fs::remove_dir_all(&build_dir).await;

    match result {
        Ok(image_tag) => {
            let duration = start.elapsed().as_secs_f64();
            send_event(
                ctx,
                "build.complete",
                serde_json::json!({
                    "deploy_id": p.deploy_id,
                    "image_tag": image_tag,
                    "duration_secs": duration,
                }),
            );
            info!(deploy = %p.deploy_id, tag = %image_tag, duration = %format!("{duration:.1}s"), "build complete");
            Ok(serde_json::json!({
                "image_tag": image_tag,
                "duration_secs": duration,
            }))
        }
        Err(e) => {
            send_event(
                ctx,
                "build.failed",
                serde_json::json!({
                    "deploy_id": p.deploy_id,
                    "error": e.to_string(),
                }),
            );
            Err(e)
        }
    }
}

async fn execute_build(
    ctx: &HandlerContext,
    p: &BuildRunParams,
    build_dir: &Path,
) -> Result<String, HandlerError> {
    // Step 1: Clone
    send_step(ctx, &p.deploy_id, "cloning", "running");
    clone_repo(
        &p.repo_url,
        p.branch.as_deref(),
        p.sha.as_deref(),
        p.token.as_deref(),
        build_dir,
    )
    .await?;
    send_step(ctx, &p.deploy_id, "cloning", "done");

    // Step 2: Detect
    send_step(ctx, &p.deploy_id, "detecting", "running");
    let detection = detect(build_dir, p.config.as_ref())
        .map_err(|e| HandlerError::Other(format!("detection failed: {e}")))?;

    send_event(
        ctx,
        "build.step",
        serde_json::json!({
            "deploy_id": p.deploy_id,
            "step": "detecting",
            "status": "done",
            "framework": detection.framework.to_string(),
            "package_manager": detection.package_manager.to_string(),
            "node_version": detection.node_version,
        }),
    );

    // Step 3: Generate Dockerfile
    send_step(ctx, &p.deploy_id, "generating", "running");
    if detection.framework != Framework::Dockerfile {
        let dockerfile_content = generate_dockerfile(&detection, p.config.as_ref())
            .map_err(|e| HandlerError::Other(format!("dockerfile generation failed: {e}")))?;
        let dockerignore = generate_dockerignore(&detection);

        tokio::fs::write(build_dir.join("Dockerfile"), &dockerfile_content).await?;
        tokio::fs::write(build_dir.join(".dockerignore"), &dockerignore).await?;

        debug!(deploy = %p.deploy_id, "generated Dockerfile for {}", detection.framework);
    } else {
        debug!(deploy = %p.deploy_id, "using existing Dockerfile");
    }
    send_step(ctx, &p.deploy_id, "generating", "done");

    // Step 4: Build image
    send_step(ctx, &p.deploy_id, "building", "running");
    let image_tag = format!("icefall/{}:{}", p.app_name, p.deploy_id);
    let context = create_build_context(build_dir)?;

    let options = BuildImageOptions {
        t: Some(image_tag.clone()),
        rm: true,
        ..Default::default()
    };

    let mut stream = ctx
        .docker
        .build_image(options, None, Some(bollard::body_full(context)));

    while let Some(result) = stream.next().await {
        let info = result.map_err(HandlerError::Docker)?;

        if let Some(ref stream_text) = info.stream {
            let trimmed = stream_text.trim();
            if !trimmed.is_empty() {
                send_event(
                    ctx,
                    "build.output",
                    serde_json::json!({
                        "deploy_id": p.deploy_id,
                        "line": trimmed,
                    }),
                );
            }
        }

        if let Some(ref detail) = info.error_detail {
            let msg = detail
                .message
                .clone()
                .unwrap_or_else(|| "unknown build error".to_string());
            return Err(HandlerError::Other(format!("docker build failed: {msg}")));
        }
    }
    send_step(ctx, &p.deploy_id, "building", "done");

    // Step 5: Tag as latest
    let latest_tag = format!("icefall/{}:latest", p.app_name);
    let tag_opts = bollard::query_parameters::TagImageOptions {
        repo: Some(format!("icefall/{}", p.app_name)),
        tag: Some("latest".to_string()),
    };
    ctx.docker
        .tag_image(&image_tag, Some(tag_opts))
        .await
        .map_err(HandlerError::Docker)?;

    Ok(image_tag)
}

async fn clone_repo(
    repo_url: &str,
    branch: Option<&str>,
    sha: Option<&str>,
    token: Option<&str>,
    work_dir: &Path,
) -> Result<(), HandlerError> {
    tokio::fs::create_dir_all(work_dir).await?;

    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");

    if let Some(branch) = branch {
        cmd.arg("--branch").arg(branch);
    }

    let url = if let Some(token) = token {
        if let Some(rest) = repo_url.strip_prefix("https://") {
            format!("https://x-access-token:{token}@{rest}")
        } else {
            repo_url.to_string()
        }
    } else {
        repo_url.to_string()
    };

    cmd.arg(&url).arg(work_dir);

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HandlerError::Other(format!(
            "git clone failed: {}",
            stderr.trim()
        )));
    }

    if let Some(sha) = sha {
        let fetch = tokio::process::Command::new("git")
            .args(["fetch", "--depth", "1", "origin", sha])
            .current_dir(work_dir)
            .output()
            .await?;

        if !fetch.status.success() {
            let stderr = String::from_utf8_lossy(&fetch.stderr);
            return Err(HandlerError::Other(format!(
                "git fetch failed: {}",
                stderr.trim()
            )));
        }

        let checkout = tokio::process::Command::new("git")
            .args(["checkout", sha])
            .current_dir(work_dir)
            .output()
            .await?;

        if !checkout.status.success() {
            let stderr = String::from_utf8_lossy(&checkout.stderr);
            return Err(HandlerError::Other(format!(
                "git checkout failed: {}",
                stderr.trim()
            )));
        }
    }

    Ok(())
}

const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".next",
    ".nuxt",
    ".output",
    "target",
    ".turbo",
    ".cache",
    "coverage",
    "dist",
];

fn create_build_context(project_dir: &Path) -> Result<Bytes, HandlerError> {
    let buf = Vec::new();
    let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::fast());
    let mut archive = tar::Builder::new(encoder);

    fn walk_and_add(
        archive: &mut tar::Builder<flate2::write::GzEncoder<Vec<u8>>>,
        dir: &Path,
        base: &Path,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if IGNORE_DIRS.contains(&name_str.as_ref()) {
                continue;
            }

            let relative = path.strip_prefix(base).unwrap_or(&path);
            if path.is_dir() {
                archive.append_dir(relative, &path)?;
                walk_and_add(archive, &path, base)?;
            } else {
                archive.append_path_with_name(&path, relative)?;
            }
        }
        Ok(())
    }

    walk_and_add(&mut archive, project_dir, project_dir)
        .map_err(|e| HandlerError::Other(format!("failed to create tar archive: {e}")))?;

    let encoder = archive
        .into_inner()
        .map_err(|e| HandlerError::Other(format!("failed to finalize tar archive: {e}")))?;
    let compressed = encoder
        .finish()
        .map_err(|e| HandlerError::Other(format!("failed to compress archive: {e}")))?;

    Ok(Bytes::from(compressed))
}

fn send_step(ctx: &HandlerContext, deploy_id: &str, step: &str, status: &str) {
    let _ = ctx.event_tx.send(AgentMessage::Event {
        event_type: "build.step".to_string(),
        data: serde_json::json!({
            "deploy_id": deploy_id,
            "step": step,
            "status": status,
        }),
    });
}

fn send_event(ctx: &HandlerContext, event_type: &str, data: Value) {
    let _ = ctx.event_tx.send(AgentMessage::Event {
        event_type: event_type.to_string(),
        data,
    });
}
