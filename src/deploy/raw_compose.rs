//! Raw Compose mode (IF-173).
//!
//! Unlike the managed [`ComposeDeployer`](crate::deploy::compose::ComposeDeployer),
//! which parses the compose file and recreates each service as an individual
//! container, raw mode hands the file straight to the host's `docker compose`
//! (or `podman compose`) CLI with no parsing or rewriting. Advanced users get
//! the full Compose feature set (build args, profiles, extends, custom
//! networking); Icefall still owns domain routing, deploy history, log capture,
//! and start/stop/restart — but not networking, health endpoints, or blue-green.
//!
//! Containers are namespaced under the Compose project `icefall-{app-slug}`, so
//! lifecycle commands (`stop`/`restart`/`down`) target the same project the
//! deploy created. The compose file and a generated `.env` live in a stable
//! per-app directory under the data root so those later commands find them.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::config::IcefallConfig;
use crate::db::models::App;
use crate::db::Database;
use crate::deploy::DeployError;
use crate::events::{EventBus, EventType};

pub struct RawComposeDeployer {
    db: Arc<dyn Database>,
    event_bus: Arc<EventBus>,
    config: Arc<IcefallConfig>,
}

impl RawComposeDeployer {
    pub fn new(
        db: Arc<dyn Database>,
        event_bus: Arc<EventBus>,
        config: Arc<IcefallConfig>,
    ) -> Self {
        Self {
            db,
            event_bus,
            config,
        }
    }

    /// The Compose project name for an app — namespaces every container the
    /// stack creates so lifecycle commands can target them as a unit.
    pub fn project_name(app: &App) -> String {
        format!("icefall-{}", slug(&app.name))
    }

    /// The per-app working directory holding `docker-compose.yml` and `.env`.
    /// Stable across deploys so stop/restart/down reuse the same files.
    fn work_dir(&self, app: &App) -> PathBuf {
        self.config
            .data_dir
            .join("raw-compose")
            .join(slug(&app.name))
    }

    /// Split the configured compose command (e.g. `"docker compose"`) into the
    /// program and its leading subcommand args.
    fn compose_argv(&self) -> (String, Vec<String>) {
        let raw = self.config.runtime.compose_command();
        let mut parts = raw.split_whitespace().map(String::from);
        let program = parts.next().unwrap_or_else(|| "docker".to_string());
        (program, parts.collect())
    }

    /// Verify the compose CLI is installed before attempting a deploy. Returns a
    /// user-facing error if `{runtime} compose version` can't be run, so the
    /// deploy log explains the missing dependency rather than a cryptic spawn
    /// failure mid-run.
    async fn preflight(&self) -> Result<(), DeployError> {
        let (program, base) = self.compose_argv();
        let output = Command::new(&program)
            .args(&base)
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
        match output {
            Ok(status) if status.success() => Ok(()),
            _ => Err(DeployError::ContainerCreate(format!(
                "`{} compose` CLI not found or not working on the host. Install the \
                 Compose plugin (Docker: `docker-compose-plugin`; Podman: `podman-compose`) \
                 to use Raw Compose mode.",
                program
            ))),
        }
    }

    /// Run a raw `compose up -d`, streaming output into the deploy log and
    /// recording the final status. The compose YAML is written verbatim — no
    /// interpolation or rewriting. App env vars are passed via `--env-file`.
    pub async fn deploy(
        &self,
        app: &App,
        deploy_id: &str,
        yaml: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(), DeployError> {
        self.preflight().await?;

        let dir = self.work_dir(app);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| DeployError::ContainerCreate(format!("create work dir: {e}")))?;

        let compose_path = dir.join("docker-compose.yml");
        let env_path = dir.join(".env");
        tokio::fs::write(&compose_path, yaml)
            .await
            .map_err(|e| DeployError::ContainerCreate(format!("write compose file: {e}")))?;
        tokio::fs::write(&env_path, render_env_file(env_vars))
            .await
            .map_err(|e| DeployError::ContainerCreate(format!("write env file: {e}")))?;

        self.emit_status(app, deploy_id, "deploying_compose");

        let project = Self::project_name(app);
        let (program, base) = self.compose_argv();
        let mut cmd = Command::new(&program);
        cmd.args(&base)
            .arg("--project-name")
            .arg(&project)
            .arg("--file")
            .arg(&compose_path)
            .arg("--env-file")
            .arg(&env_path)
            .current_dir(&dir)
            .arg("up")
            .arg("-d")
            .arg("--remove-orphans");

        self.run_streaming(&mut cmd, deploy_id).await?;

        // Mark running and record the drift hash, mirroring the managed path so
        // deploy history and drift detection behave the same in raw mode.
        let _ =
            crate::deploy::retry_state_write("raw-compose update_deploy_status running", || {
                self.db.update_deploy_status(deploy_id, "running", None)
            })
            .await;
        self.emit_status(app, deploy_id, "running");

        let env_pairs: Vec<(String, String)> = env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let domain_list: Vec<String> = self
            .db
            .list_domains(&app.id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.domain)
            .collect();
        let hash = crate::deploy::drift::compute_config_hash(app, &env_pairs, &domain_list);
        let _ = self.db.update_deploy_config_hash(deploy_id, &hash).await;

        Ok(())
    }

    /// `compose stop` — halts the stack without removing containers.
    pub async fn stop(&self, app: &App) -> Result<(), DeployError> {
        self.lifecycle(app, &["stop"]).await
    }

    /// `compose restart` — restarts the running stack.
    pub async fn restart(&self, app: &App) -> Result<(), DeployError> {
        self.lifecycle(app, &["restart"]).await
    }

    /// `compose down` — stops and removes the stack's containers and networks.
    /// Used on teardown / app delete.
    pub async fn down(&self, app: &App) -> Result<(), DeployError> {
        self.lifecycle(app, &["down"]).await
    }

    /// Whether the stack's containers are running, by Compose project name.
    /// Raw mode can't probe individual health endpoints, so "running" means
    /// `compose ps` reports at least one service in the running state.
    pub async fn is_running(&self, app: &App) -> bool {
        let project = Self::project_name(app);
        let (program, base) = self.compose_argv();
        let dir = self.work_dir(app);
        // `compose ps -q` prints one container id per running service. Empty
        // output ⇒ nothing running.
        let output = Command::new(&program)
            .args(&base)
            .arg("--project-name")
            .arg(&project)
            .current_dir(&dir)
            .arg("ps")
            .arg("-q")
            .output()
            .await;
        matches!(output, Ok(o) if o.status.success() && !o.stdout.is_empty())
    }

    /// Run a compose subcommand against the app's project, no output streaming.
    async fn lifecycle(&self, app: &App, args: &[&str]) -> Result<(), DeployError> {
        let project = Self::project_name(app);
        let dir = self.work_dir(app);
        let compose_path = dir.join("docker-compose.yml");
        let (program, base) = self.compose_argv();

        let mut cmd = Command::new(&program);
        cmd.args(&base)
            .arg("--project-name")
            .arg(&project)
            .arg("--file")
            .arg(&compose_path)
            .current_dir(&dir)
            .args(args);

        let status = cmd.status().await.map_err(|e| {
            DeployError::ContainerCreate(format!("compose {}: {e}", args.join(" ")))
        })?;
        if !status.success() {
            return Err(DeployError::ContainerCreate(format!(
                "`compose {}` exited with {status}",
                args.join(" ")
            )));
        }
        Ok(())
    }

    /// Spawn a command, streaming each stdout/stderr line into the deploy log as
    /// a build-output event, then fail if it exits non-zero.
    async fn run_streaming(&self, cmd: &mut Command, deploy_id: &str) -> Result<(), DeployError> {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| DeployError::ContainerCreate(format!("spawn compose: {e}")))?;

        // Drain stdout and stderr concurrently so a chatty stream on one pipe
        // can't deadlock the other.
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (s_out, s_err) = (self.event_bus.clone(), self.event_bus.clone());
        let (id_out, id_err) = (deploy_id.to_string(), deploy_id.to_string());

        let out_task = tokio::spawn(async move {
            if let Some(out) = stdout {
                let mut lines = BufReader::new(out).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    emit_log(&s_out, &id_out, &line);
                }
            }
        });
        let err_task = tokio::spawn(async move {
            if let Some(err) = stderr {
                let mut lines = BufReader::new(err).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    emit_log(&s_err, &id_err, &line);
                }
            }
        });

        let status = child
            .wait()
            .await
            .map_err(|e| DeployError::ContainerCreate(format!("await compose: {e}")))?;
        let _ = out_task.await;
        let _ = err_task.await;

        if !status.success() {
            return Err(DeployError::ContainerCreate(format!(
                "`compose up` exited with {status}"
            )));
        }
        Ok(())
    }

    fn emit_status(&self, app: &App, deploy_id: &str, status: &str) {
        self.event_bus.emit(
            EventType::DeployStatus,
            Some(&app.id),
            Some(deploy_id),
            serde_json::json!({ "status": status, "compose": true, "raw": true }),
        );
    }
}

/// Emit a single line of compose output to the deploy log.
fn emit_log(bus: &EventBus, deploy_id: &str, line: &str) {
    bus.emit(
        EventType::BuildStepOutput,
        None,
        Some(deploy_id),
        serde_json::json!({ "output": line }),
    );
}

/// Render an env map as `KEY=value` lines for `--env-file`. Compose reads this
/// file literally — values are not shell-evaluated, so no quoting is needed, but
/// newlines in a value would corrupt the file, so they are stripped.
fn render_env_file(env_vars: &HashMap<String, String>) -> String {
    let mut keys: Vec<&String> = env_vars.keys().collect();
    keys.sort(); // Deterministic output so the file (and its drift) is stable.
    let mut out = String::new();
    for key in keys {
        let value = env_vars[key].replace(['\n', '\r'], " ");
        out.push_str(key);
        out.push('=');
        out.push_str(&value);
        out.push('\n');
    }
    out
}

/// Lowercase, hyphenated slug of an app name for project/dir naming. Mirrors the
/// `icefall-{name}` convention used elsewhere but sanitizes to compose-safe
/// characters.
fn slug(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_sanitizes() {
        assert_eq!(slug("My App"), "my-app");
        assert_eq!(slug("  Weird/Name_123  "), "weird-name-123");
        assert_eq!(slug("already-ok"), "already-ok");
    }

    #[test]
    fn env_file_is_sorted_and_newline_safe() {
        let mut env = HashMap::new();
        env.insert("B_KEY".to_string(), "two".to_string());
        env.insert("A_KEY".to_string(), "line1\nline2".to_string());
        let rendered = render_env_file(&env);
        assert_eq!(rendered, "A_KEY=line1 line2\nB_KEY=two\n");
    }
}
