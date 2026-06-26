//! CLI-based image build with true line-by-line streaming.
//!
//! The runtime's REST `/build` endpoint (what `bollard` uses) does NOT stream
//! per-step `RUN` output under Podman — it only sends progress and the final
//! error, so the actual `npm`/`vite`/compiler logs never reach the deploy log.
//! Shelling out to the `podman build` (or `docker build`) CLI and reading its
//! merged stdout/stderr gives us the full, live build output instead.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::config::ContainerRuntime;
use crate::docker::{DockerClient, DockerError};

impl DockerClient {
    /// The runtime CLI binary name for the connected runtime.
    fn cli_binary(&self) -> &'static str {
        match self.quirks().runtime {
            ContainerRuntime::Podman => "podman",
            ContainerRuntime::Docker => "docker",
        }
    }

    /// Build an image from a context directory using the runtime CLI, invoking
    /// `on_line` for every output line as it is produced (merged stdout+stderr).
    ///
    /// The directory must already contain the Dockerfile and source. Returns the
    /// build error (including the last lines of output) on non-zero exit.
    pub async fn build_image_cli<F>(
        &self,
        tag: &str,
        context_dir: &Path,
        no_cache: bool,
        mut on_line: F,
    ) -> Result<(), DockerError>
    where
        F: FnMut(&str) + Send,
    {
        let bin = self.cli_binary();
        let mut cmd = Command::new(bin);
        cmd.arg("build").arg("-t").arg(tag).arg("--force-rm");
        if no_cache {
            cmd.arg("--no-cache");
        }
        cmd.arg(context_dir);

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let mut child = cmd
            .spawn()
            .map_err(|e| DockerError::BuildFailed(format!("failed to spawn `{bin} build`: {e}")))?;

        // Merge stdout and stderr into a single ordered stream of lines. Build
        // tools write progress to stderr and program output to stdout; the user
        // wants both interleaved as the terminal would show them.
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let mut tail: Vec<String> = Vec::with_capacity(20);
        let push_tail = |line: &str, tail: &mut Vec<String>| {
            tail.push(line.to_string());
            if tail.len() > 20 {
                tail.remove(0);
            }
        };

        let mut out_lines = stdout
            .map(|s| BufReader::new(s).lines())
            .expect("stdout piped");
        let mut err_lines = stderr
            .map(|s| BufReader::new(s).lines())
            .expect("stderr piped");

        let mut out_open = true;
        let mut err_open = true;
        while out_open || err_open {
            tokio::select! {
                // Disabling a branch once its stream closes prevents busy-looping
                // on the perpetual `Ok(None)` a finished reader returns.
                line = out_lines.next_line(), if out_open => match line {
                    Ok(Some(l)) => { push_tail(&l, &mut tail); on_line(&l); }
                    Ok(None) | Err(_) => out_open = false,
                },
                line = err_lines.next_line(), if err_open => match line {
                    Ok(Some(l)) => { push_tail(&l, &mut tail); on_line(&l); }
                    Ok(None) | Err(_) => err_open = false,
                },
            }
        }

        let status = child.wait().await.map_err(|e| {
            DockerError::BuildFailed(format!("`{bin} build` did not complete: {e}"))
        })?;

        if !status.success() {
            let code = status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string());
            let detail = tail.join("\n");
            return Err(DockerError::BuildFailed(format!(
                "`{bin} build` exited with status {code}\n{detail}"
            )));
        }

        Ok(())
    }
}
