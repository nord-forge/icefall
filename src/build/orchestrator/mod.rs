mod context;
mod tests;

use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::StreamExt;

use crate::build::detect::detect;
use crate::build::dockerfile::{generate_dockerfile, generate_dockerignore};
use crate::build::git::{clone_repo, GitCloneOptions};
use crate::build::{BuildConfig, BuildError, BuildResult, BuildStep, BuildStepStatus, Framework};
use crate::config::IcefallConfig;
use crate::db::models::App;
use crate::db::Database;
use crate::docker::DockerClient;

use context::{create_build_context, finish_step, new_step, redact_secrets};

pub struct BuildOrchestrator {
    docker: Arc<DockerClient>,
    db: Arc<dyn Database>,
    config: Arc<IcefallConfig>,
}

impl BuildOrchestrator {
    pub fn new(
        docker: Arc<DockerClient>,
        db: Arc<dyn Database>,
        config: Arc<IcefallConfig>,
    ) -> Self {
        Self { docker, db, config }
    }

    pub async fn build(
        &self,
        deploy_id: &str,
        app: &App,
        build_config: Option<BuildConfig>,
        no_cache: bool,
    ) -> Result<BuildResult, BuildError> {
        let start = Instant::now();
        let mut steps: Vec<BuildStep> = Vec::with_capacity(6);
        let mut all_output: Vec<String> = Vec::with_capacity(256);

        self.db
            .update_deploy_status(deploy_id, "building", None)
            .await?;

        // Collect secrets for redaction
        let secrets = self.collect_secrets(deploy_id).await;

        // Step 1: Clone
        let mut step = new_step("Cloning repository");
        let work_dir = self.config.data_dir.join("builds").join(deploy_id);

        let git_repo = app
            .git_repo
            .as_deref()
            .ok_or_else(|| BuildError::GitClone("app has no git_repo configured".to_string()))?;

        let clone_opts = GitCloneOptions {
            repo_url: git_repo.to_string(),
            branch: Some(app.git_branch.clone()),
            sha: None,
            ssh_key_path: None,
            token: None,
        };

        match clone_repo(&clone_opts, &work_dir).await {
            Ok(result) => {
                let msg = format!(
                    "Cloned {} at {}",
                    git_repo,
                    &result.resolved_sha[..8.min(result.resolved_sha.len())]
                );
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Done);
            }
            Err(e) => {
                let msg = format!("Clone failed: {e}");
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                steps.push(step);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(e);
            }
        }
        steps.push(step);

        if self.is_cancelled(deploy_id).await {
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
            return Err(BuildError::Cancelled);
        }

        // Step 2: Detect
        let mut step = new_step("Detecting framework");
        let detection = match detect(&work_dir, build_config.as_ref()) {
            Ok(det) => {
                let msg = format!(
                    "Detected {} with {} (node {})",
                    det.framework, det.package_manager, det.node_version
                );
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Done);
                det
            }
            Err(e) => {
                let msg = format!("Detection failed: {e}");
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                steps.push(step);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(e.into());
            }
        };
        steps.push(step);

        if self.is_cancelled(deploy_id).await {
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
            return Err(BuildError::Cancelled);
        }

        // Step 3: Generate Dockerfile
        let mut step = new_step("Generating Dockerfile");
        let uses_existing_dockerfile = detection.framework == Framework::Dockerfile;

        if !uses_existing_dockerfile {
            match generate_dockerfile(&detection, build_config.as_ref()) {
                Ok(dockerfile_content) => {
                    let dockerignore = generate_dockerignore(&detection);

                    if let Err(e) =
                        tokio::fs::write(work_dir.join("Dockerfile"), &dockerfile_content).await
                    {
                        let msg = format!("Failed to write Dockerfile: {e}");
                        step.output.push(msg.clone());
                        all_output.push(msg);
                        finish_step(&mut step, BuildStepStatus::Failed);
                        steps.push(step);
                        self.fail_deploy(deploy_id, &all_output).await;
                        return Err(BuildError::Io(e));
                    }
                    let _ = tokio::fs::write(work_dir.join(".dockerignore"), &dockerignore).await;

                    let msg = format!("Generated Dockerfile for {}", detection.framework);
                    step.output.push(msg.clone());
                    all_output.push(msg);
                    finish_step(&mut step, BuildStepStatus::Done);
                }
                Err(e) => {
                    let msg = format!("Dockerfile generation failed: {e}");
                    step.output.push(msg.clone());
                    all_output.push(msg);
                    finish_step(&mut step, BuildStepStatus::Failed);
                    steps.push(step);
                    self.fail_deploy(deploy_id, &all_output).await;
                    return Err(e.into());
                }
            }
        } else {
            step.output.push("Using existing Dockerfile".to_string());
            all_output.push("Using existing Dockerfile".to_string());
            finish_step(&mut step, BuildStepStatus::Done);
        }
        steps.push(step);

        if self.is_cancelled(deploy_id).await {
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
            return Err(BuildError::Cancelled);
        }

        // Step 4: Build image
        let mut step = new_step("Building container image");
        let image_tag = format!("icefall/{}:{}", app.name, deploy_id);

        let context = match create_build_context(&work_dir) {
            Ok(ctx) => ctx,
            Err(e) => {
                let msg = format!("Failed to create build context: {e}");
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                steps.push(step);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(e);
            }
        };

        let timeout_secs = build_config
            .as_ref()
            .and_then(|c| c.build_timeout_secs)
            .unwrap_or(self.config.build_timeout_secs);

        if no_cache {
            all_output.push("Force rebuild: build cache disabled".to_string());
        }

        let build_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            self.stream_build(&image_tag, context, &secrets, no_cache, deploy_id),
        )
        .await;

        match build_result {
            Ok(Ok(lines)) => {
                step.output.extend(lines.iter().cloned());
                all_output.extend(lines);
                finish_step(&mut step, BuildStepStatus::Done);
            }
            Ok(Err(e)) => {
                let msg = format!("Build failed: {e}");
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                steps.push(step);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(e);
            }
            Err(_) => {
                let msg = format!("Build timed out after {timeout_secs}s");
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                steps.push(step);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(BuildError::Timeout(timeout_secs));
            }
        }
        steps.push(step);

        // Step 5: Tag
        let mut step = new_step("Tagging image");
        let latest_tag = format!("icefall/{}:latest", app.name);

        if let Err(e) = self
            .docker
            .tag_image(&image_tag, &format!("icefall/{}", app.name), "latest")
            .await
        {
            let msg = format!("Tagging failed: {e}");
            step.output.push(msg.clone());
            all_output.push(msg);
            finish_step(&mut step, BuildStepStatus::Failed);
            steps.push(step);
            self.fail_deploy(deploy_id, &all_output).await;
            return Err(BuildError::Docker(e));
        }

        step.output.push(format!("Tagged as {latest_tag}"));
        all_output.push(format!("Tagged as {latest_tag}"));
        finish_step(&mut step, BuildStepStatus::Done);
        steps.push(step);

        // Step 6: Cleanup
        let mut step = new_step("Cleaning up");
        let _ = tokio::fs::remove_dir_all(&work_dir).await;

        let keep = build_config
            .as_ref()
            .and_then(|c| c.keep_images)
            .unwrap_or(self.config.keep_images);

        match self.cleanup_old_images(&app.name, keep).await {
            Ok(removed) => {
                if !removed.is_empty() {
                    let msg = format!("Removed {} old image(s)", removed.len());
                    step.output.push(msg.clone());
                    all_output.push(msg);
                }
            }
            Err(e) => {
                tracing::warn!("Image cleanup failed: {e}");
            }
        }
        finish_step(&mut step, BuildStepStatus::Done);
        steps.push(step);

        // Update deploy record
        let log = all_output.join("\n");
        let _ = self
            .db
            .update_deploy_status(deploy_id, "deploying", Some(&log))
            .await;

        let total_duration_secs = start.elapsed().as_secs_f64();

        Ok(BuildResult {
            image_ref: image_tag.clone(),
            image_tags: vec![image_tag, latest_tag],
            detection,
            steps,
            total_duration_secs,
        })
    }

    async fn stream_build(
        &self,
        tag: &str,
        context: Bytes,
        secrets: &[String],
        no_cache: bool,
        deploy_id: &str,
    ) -> Result<Vec<String>, BuildError> {
        let mut lines = Vec::with_capacity(256);
        let mut stream = self.docker.build_image(tag, context, no_cache);
        let mut line_count = 0u32;

        while let Some(result) = stream.next().await {
            let info = result?;

            if let Some(stream_msg) = &info.stream {
                let line = stream_msg.trim_end();
                if !line.is_empty() {
                    lines.push(redact_secrets(line, secrets));
                }
            }

            if let Some(ref detail) = info.error_detail {
                let msg = detail
                    .message
                    .clone()
                    .unwrap_or_else(|| "unknown build error".to_string());
                return Err(BuildError::DockerBuild(msg));
            }

            line_count += 1;
            if line_count % 50 == 0 && self.is_cancelled(deploy_id).await {
                return Err(BuildError::Cancelled);
            }
        }

        Ok(lines)
    }

    async fn is_cancelled(&self, deploy_id: &str) -> bool {
        matches!(
            self.db.get_deploy(deploy_id).await,
            Ok(Some(d)) if d.status == "cancelled"
        )
    }

    async fn fail_deploy(&self, deploy_id: &str, output: &[String]) {
        let tail: Vec<&str> = output
            .iter()
            .rev()
            .take(50)
            .map(std::string::String::as_str)
            .collect();
        let log = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
        let _ = self
            .db
            .update_deploy_status(deploy_id, "failed", Some(&log))
            .await;
    }

    async fn collect_secrets(&self, deploy_id: &str) -> Vec<String> {
        let Ok(Some(deploy)) = self.db.get_deploy(deploy_id).await else {
            return Vec::new();
        };

        match self.db.get_env_vars(&deploy.environment_id).await {
            Ok(vars) => vars
                .into_iter()
                .filter(|v| !v.value.is_empty())
                .map(|v| v.value)
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn cleanup_old_images(
        &self,
        app_name: &str,
        keep: usize,
    ) -> Result<Vec<String>, BuildError> {
        let reference = format!("icefall/{app_name}");
        let mut images = self.docker.list_images(Some(&reference)).await?;

        if images.len() <= keep {
            return Ok(Vec::new());
        }

        images.sort_by_key(|img| std::cmp::Reverse(img.created));

        let to_remove = &images[keep..];
        let mut removed = Vec::new();

        for image in to_remove {
            let id = image.id.strip_prefix("sha256:").unwrap_or(&image.id);
            let tag = image
                .repo_tags
                .first()
                .cloned()
                .unwrap_or_else(|| id.to_string());
            match self.docker.remove_image(&tag).await {
                Ok(()) => removed.push(tag),
                Err(e) => tracing::warn!("Failed to remove image {tag}: {e}"),
            }
        }

        Ok(removed)
    }
}
