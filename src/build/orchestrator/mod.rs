mod context;
mod tests;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::build::detect::detect;
use crate::build::dockerfile::{generate_dockerfile, generate_dockerignore};
use crate::build::git::{clone_repo, GitCloneOptions};
use crate::build::{BuildConfig, BuildError, BuildResult, BuildStep, BuildStepStatus, Framework};
use crate::config::IcefallConfig;
use crate::db::models::App;
use crate::db::Database;
use crate::docker::DockerClient;
use crate::events::{EventBus, EventType};

use context::{finish_step, new_step, redact_secrets, sanitize_image_name};

pub struct BuildOrchestrator {
    docker: Arc<DockerClient>,
    db: Arc<dyn Database>,
    config: Arc<IcefallConfig>,
    events: Arc<EventBus>,
}

impl BuildOrchestrator {
    pub fn new(
        docker: Arc<DockerClient>,
        db: Arc<dyn Database>,
        config: Arc<IcefallConfig>,
        events: Arc<EventBus>,
    ) -> Self {
        Self {
            docker,
            db,
            config,
            events,
        }
    }

    /// Announce a build step starting, so the live view renders it immediately.
    fn emit_step_start(&self, app_id: &str, deploy_id: &str, name: &str) {
        self.events.emit(
            EventType::BuildStepStart,
            Some(app_id),
            Some(deploy_id),
            serde_json::json!({ "name": name }),
        );
    }

    /// Stream a single output line for the current step to live subscribers.
    fn emit_step_output(&self, app_id: &str, deploy_id: &str, line: &str) {
        self.events.emit(
            EventType::BuildStepOutput,
            Some(app_id),
            Some(deploy_id),
            serde_json::json!({ "line": line }),
        );
    }

    fn emit_step_complete(&self, app_id: &str, deploy_id: &str, status: &str) {
        self.events.emit(
            EventType::BuildStepComplete,
            Some(app_id),
            Some(deploy_id),
            serde_json::json!({ "status": status }),
        );
    }

    /// Persist the accumulated structured steps so a page load mid-build shows
    /// the same step view the live SSE stream is building. Best-effort.
    async fn persist_steps(&self, deploy_id: &str, steps: &[BuildStep], current: &BuildStep) {
        let mut snapshot: Vec<&BuildStep> = steps.iter().collect();
        snapshot.push(current);
        if let Ok(json) = serde_json::to_string(&snapshot) {
            let _ = self.db.set_deploy_log(deploy_id, &json).await;
        }
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
        // Flip the UI to "building" the instant the build starts.
        self.events.emit(
            EventType::DeployStatus,
            Some(&app.id),
            Some(deploy_id),
            serde_json::json!({ "status": "building" }),
        );

        let secrets = self.collect_secrets(deploy_id).await;

        // Step 1: Clone
        let mut step = new_step("Cloning repository");
        self.emit_step_start(&app.id, deploy_id, &step.name);
        let work_dir = self.config.data_dir.join("builds").join(deploy_id);

        let git_repo = app
            .git_repo
            .as_deref()
            .ok_or_else(|| BuildError::GitClone("app has no git_repo configured".to_string()))?;

        // Obtain a GitHub installation access token for cloning private repos
        let github_token = self.resolve_github_token(git_repo).await;

        let clone_opts = GitCloneOptions {
            repo_url: git_repo.to_string(),
            branch: Some(app.git_branch.clone()),
            sha: None,
            ssh_key_path: None,
            token: github_token,
            submodules: app.git_submodules_enabled,
            lfs: app.git_lfs_enabled,
            shallow: app.git_shallow_clone,
        };

        match clone_repo(&clone_opts, &work_dir).await {
            Ok(result) => {
                let msg = format!(
                    "Cloned {} at {}",
                    git_repo,
                    &result.resolved_sha[..8.min(result.resolved_sha.len())]
                );
                self.emit_step_output(&app.id, deploy_id, &msg);
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Done);
                self.emit_step_complete(&app.id, deploy_id, "done");
            }
            Err(e) => {
                let msg = format!("Clone failed: {e}");
                self.emit_step_output(&app.id, deploy_id, &msg);
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                self.emit_step_complete(&app.id, deploy_id, "failed");
                steps.push(step);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(e);
            }
        }
        steps.push(step);

        let effective_dir = if let Some(ref base_dir) = app.base_directory {
            let sub = work_dir.join(base_dir);
            if !sub.exists() {
                let msg = format!("Base directory '{base_dir}' not found in repository");
                all_output.push(msg);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(BuildError::GitClone(format!(
                    "base directory '{base_dir}' not found"
                )));
            }
            all_output.push(format!("Using base directory: {base_dir}"));
            sub
        } else {
            work_dir.clone()
        };
        if self.is_cancelled(deploy_id).await {
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
            return Err(BuildError::Cancelled);
        }

        // Step 2: Detect
        let mut step = new_step("Detecting framework");
        self.emit_step_start(&app.id, deploy_id, &step.name);
        let detection = match detect(&effective_dir, build_config.as_ref()) {
            Ok(det) => {
                let msg = format!(
                    "Detected {} with {} (node {})",
                    det.framework, det.package_manager, det.node_version
                );
                self.emit_step_output(&app.id, deploy_id, &msg);
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Done);
                self.emit_step_complete(&app.id, deploy_id, "done");
                det
            }
            Err(e) => {
                let msg = format!("Detection failed: {e}");
                self.emit_step_output(&app.id, deploy_id, &msg);
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                self.emit_step_complete(&app.id, deploy_id, "failed");
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
        self.emit_step_start(&app.id, deploy_id, &step.name);
        let uses_existing_dockerfile = detection.framework == Framework::Dockerfile;

        if !uses_existing_dockerfile {
            match generate_dockerfile(&detection, build_config.as_ref()) {
                Ok(dockerfile_content) => {
                    let dockerignore = generate_dockerignore(&detection);

                    if let Err(e) =
                        tokio::fs::write(effective_dir.join("Dockerfile"), &dockerfile_content)
                            .await
                    {
                        let msg = format!("Failed to write Dockerfile: {e}");
                        self.emit_step_output(&app.id, deploy_id, &msg);
                        step.output.push(msg.clone());
                        all_output.push(msg);
                        finish_step(&mut step, BuildStepStatus::Failed);
                        self.emit_step_complete(&app.id, deploy_id, "failed");
                        steps.push(step);
                        self.fail_deploy(deploy_id, &all_output).await;
                        return Err(BuildError::Io(e));
                    }
                    let _ =
                        tokio::fs::write(effective_dir.join(".dockerignore"), &dockerignore).await;

                    let msg = format!("Generated Dockerfile for {}", detection.framework);
                    self.emit_step_output(&app.id, deploy_id, &msg);
                    step.output.push(msg.clone());
                    all_output.push(msg);
                    finish_step(&mut step, BuildStepStatus::Done);
                    self.emit_step_complete(&app.id, deploy_id, "done");
                }
                Err(e) => {
                    let msg = format!("Dockerfile generation failed: {e}");
                    self.emit_step_output(&app.id, deploy_id, &msg);
                    step.output.push(msg.clone());
                    all_output.push(msg);
                    finish_step(&mut step, BuildStepStatus::Failed);
                    self.emit_step_complete(&app.id, deploy_id, "failed");
                    steps.push(step);
                    self.fail_deploy(deploy_id, &all_output).await;
                    return Err(e.into());
                }
            }
        } else {
            let msg = "Using existing Dockerfile".to_string();
            self.emit_step_output(&app.id, deploy_id, &msg);
            step.output.push(msg.clone());
            all_output.push(msg);
            finish_step(&mut step, BuildStepStatus::Done);
            self.emit_step_complete(&app.id, deploy_id, "done");
        }
        steps.push(step);

        if self.is_cancelled(deploy_id).await {
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
            return Err(BuildError::Cancelled);
        }

        // Step 4: Build image
        let mut step = new_step("Building container image");
        self.emit_step_start(&app.id, deploy_id, &step.name);
        // Sanitize the name for the image reference — names are validated at
        // create time, but defend against legacy/edge values producing an
        // "invalid reference format" error here.
        let image_name = sanitize_image_name(&app.name);
        let image_tag = format!("icefall/{image_name}:{deploy_id}");

        let timeout_secs = build_config
            .as_ref()
            .and_then(|c| c.build_timeout_secs)
            .unwrap_or(self.config.build_timeout_secs);

        if no_cache {
            let msg = "Force rebuild: build cache disabled".to_string();
            self.emit_step_output(&app.id, deploy_id, &msg);
            step.output.push(msg.clone());
            all_output.push(msg);
        }

        let build_result = self
            .stream_build_cli(
                &image_tag,
                &effective_dir,
                &secrets,
                no_cache,
                deploy_id,
                &app.id,
                timeout_secs,
                &steps,
                &mut step,
            )
            .await;

        // Fold the streamed lines into the running totals regardless of outcome,
        // so the failing RUN step's npm/vite logs reach the deploy log.
        all_output.extend(step.output.iter().cloned());

        match build_result {
            Ok(()) => {
                finish_step(&mut step, BuildStepStatus::Done);
                self.emit_step_complete(&app.id, deploy_id, "done");
            }
            Err(e) => {
                let msg = match &e {
                    BuildError::Timeout(s) => format!("Build timed out after {s}s"),
                    other => format!("Build failed: {other}"),
                };
                self.emit_step_output(&app.id, deploy_id, &msg);
                step.output.push(msg.clone());
                all_output.push(msg);
                finish_step(&mut step, BuildStepStatus::Failed);
                self.emit_step_complete(&app.id, deploy_id, "failed");
                steps.push(step);
                self.fail_deploy(deploy_id, &all_output).await;
                return Err(e);
            }
        }
        steps.push(step);

        // Step 5: Tag
        let mut step = new_step("Tagging image");
        self.emit_step_start(&app.id, deploy_id, &step.name);
        let latest_tag = format!("icefall/{image_name}:latest");

        if let Err(e) = self
            .docker
            .tag_image(&image_tag, &format!("icefall/{image_name}"), "latest")
            .await
        {
            let msg = format!("Tagging failed: {e}");
            self.emit_step_output(&app.id, deploy_id, &msg);
            step.output.push(msg.clone());
            all_output.push(msg);
            finish_step(&mut step, BuildStepStatus::Failed);
            self.emit_step_complete(&app.id, deploy_id, "failed");
            steps.push(step);
            self.fail_deploy(deploy_id, &all_output).await;
            return Err(BuildError::Docker(e));
        }

        let msg = format!("Tagged as {latest_tag}");
        self.emit_step_output(&app.id, deploy_id, &msg);
        step.output.push(msg.clone());
        all_output.push(msg);
        finish_step(&mut step, BuildStepStatus::Done);
        self.emit_step_complete(&app.id, deploy_id, "done");
        steps.push(step);

        // Step 6: Cleanup
        let mut step = new_step("Cleaning up");
        self.emit_step_start(&app.id, deploy_id, &step.name);
        let _ = tokio::fs::remove_dir_all(&work_dir).await;

        let keep = build_config
            .as_ref()
            .and_then(|c| c.keep_images)
            .unwrap_or(self.config.keep_images);

        match self.cleanup_old_images(&app.name, keep).await {
            Ok(removed) => {
                if !removed.is_empty() {
                    let msg = format!("Removed {} old image(s)", removed.len());
                    self.emit_step_output(&app.id, deploy_id, &msg);
                    step.output.push(msg.clone());
                    all_output.push(msg);
                }
            }
            Err(e) => {
                tracing::warn!("Image cleanup failed: {e}");
            }
        }
        finish_step(&mut step, BuildStepStatus::Done);
        self.emit_step_complete(&app.id, deploy_id, "done");
        steps.push(step);

        // Persist the structured steps as the build log so a page load renders
        // the same rich step view the live SSE stream built, then advance state.
        let log = serde_json::to_string(&steps).unwrap_or_else(|_| all_output.join("\n"));
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

    /// Stream the image build via the runtime CLI, pushing every line into the
    /// step (and emitting it live + persisting periodically) as it arrives. The
    /// CLI path is used because Podman's REST build endpoint does not stream
    /// per-step `RUN` output — only the CLI surfaces the real npm/compiler logs.
    // Streaming a build needs the tag, context, secrets, cache flag, both ids,
    // a timeout, and the step accumulator — splitting into a struct would only
    // scatter this one call site's locals.
    #[expect(clippy::too_many_arguments)]
    async fn stream_build_cli(
        &self,
        tag: &str,
        context_dir: &std::path::Path,
        secrets: &[String],
        no_cache: bool,
        deploy_id: &str,
        app_id: &str,
        timeout_secs: u64,
        prior_steps: &[BuildStep],
        step: &mut BuildStep,
    ) -> Result<(), BuildError> {
        use std::sync::Mutex;

        // Shared line buffer the sync `on_line` closure appends to, drained by a
        // concurrent persister so a page load mid-build sees output so far.
        let buf: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::with_capacity(256)));
        let secrets = secrets.to_vec();

        let build = {
            let buf = buf.clone();
            let events = self.events.clone();
            let app_id = app_id.to_string();
            let deploy_id_owned = deploy_id.to_string();
            self.docker
                .build_image_cli(tag, context_dir, no_cache, move |raw| {
                    let line = redact_secrets(raw.trim_end(), &secrets);
                    if line.is_empty() {
                        return;
                    }
                    events.emit(
                        EventType::BuildStepOutput,
                        Some(&app_id),
                        Some(&deploy_id_owned),
                        serde_json::json!({ "line": line }),
                    );
                    if let Ok(mut g) = buf.lock() {
                        g.push(line);
                    }
                })
        };

        // Persist the accumulated step snapshot every second so a refresh shows
        // progress, and abort the build if the deploy is cancelled meanwhile.
        let persist = async {
            let mut last_len = 0usize;
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let snapshot: Vec<String> = buf.lock().map(|g| g.clone()).unwrap_or_default();
                if snapshot.len() != last_len {
                    last_len = snapshot.len();
                    let mut live = step.clone();
                    live.output = snapshot;
                    self.persist_steps(deploy_id, prior_steps, &live).await;
                }
                if self.is_cancelled(deploy_id).await {
                    return Err(BuildError::Cancelled);
                }
            }
        };

        let outcome = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
            tokio::select! {
                r = build => r.map_err(BuildError::from),
                // `persist` only returns on cancellation; otherwise it loops
                // forever and `build` completing wins the select.
                e = persist => e,
            }
        })
        .await;

        // Drain the buffer into the step regardless of outcome.
        if let Ok(g) = buf.lock() {
            step.output.extend(g.iter().cloned());
        }

        match outcome {
            Ok(r) => r,
            Err(_) => Err(BuildError::Timeout(timeout_secs)),
        }
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

    /// Resolve a GitHub installation access token for the given repo URL.
    /// Returns None if no matching installation is found or token generation fails.
    async fn resolve_github_token(&self, repo_url: &str) -> Option<String> {
        if !repo_url.contains("github.com") {
            return None;
        }

        let installations = self.db.list_github_installations().await.ok()?;

        for installation in &installations {
            let app = match self
                .db
                .get_github_app_for_installation(installation.installation_id)
                .await
            {
                Ok(Some(app)) => app,
                _ => continue,
            };

            let jwt = match crate::github::auth::generate_jwt(app.app_id, &app.private_key) {
                Ok(jwt) => jwt,
                Err(e) => {
                    tracing::warn!("Failed to generate JWT for GitHub App {}: {e}", app.app_id);
                    continue;
                }
            };

            let client = crate::github::client::GitHubClient::new(&app.api_url);
            match client
                .get_installation_token(&jwt, installation.installation_id)
                .await
            {
                Ok(token) => {
                    tracing::info!(
                        installation_id = installation.installation_id,
                        app_name = %app.name,
                        "Using GitHub App installation token for git clone"
                    );
                    return Some(token.token);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to get installation token for installation {}: {e}",
                        installation.installation_id
                    );
                    continue;
                }
            }
        }

        None
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
