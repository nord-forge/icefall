use std::path::{Path, PathBuf};

use crate::build::PackageManager;

/// Get the install command for a package manager.
pub(super) fn install_command(pm: &PackageManager) -> String {
    match pm {
        PackageManager::Npm => "npm ci".to_string(),
        PackageManager::Yarn => "yarn install --frozen-lockfile".to_string(),
        PackageManager::Pnpm => "pnpm install --frozen-lockfile".to_string(),
        PackageManager::Bun => "bun install --frozen-lockfile".to_string(),
    }
}

/// Run a shell command in a directory, returning an error if it fails.
pub(super) async fn run_command(cmd: &str, dir: &Path) -> Result<String, String> {
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(dir)
        .output()
        .await
        .map_err(|e| format!("failed to spawn: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "exit code {}: {}\n{}",
            output.status.code().unwrap_or(-1),
            stderr.trim(),
            stdout.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Recursively copy a directory.
pub(super) async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(dst).await?;

    let mut entries = tokio::fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await?;
        }
    }

    Ok(())
}

/// Atomically switch a symlink using `ln -sfn`.
pub(super) async fn atomic_symlink(target: &Path, link: &Path) -> Result<(), std::io::Error> {
    let output = tokio::process::Command::new("ln")
        .arg("-sfn")
        .arg(target)
        .arg(link)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!("ln -sfn failed: {stderr}")));
    }

    Ok(())
}

/// Remove old deploy directories, keeping the most recent `keep` count plus the current deploy.
pub(super) async fn cleanup_old_deploys(
    sites_dir: &Path,
    current_deploy_id: &str,
    keep: usize,
) -> Result<(), std::io::Error> {
    let mut entries = tokio::fs::read_dir(sites_dir).await?;
    let mut deploy_dirs: Vec<(String, PathBuf)> = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip the "current" symlink
        if name == "current" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            deploy_dirs.push((name, path));
        }
    }

    // Sort by name descending (UUIDs are v7 so they sort chronologically)
    deploy_dirs.sort_by(|a, b| b.0.cmp(&a.0));

    // Keep the current deploy and the most recent `keep` directories
    let mut kept = 0usize;
    for (name, path) in &deploy_dirs {
        if name == current_deploy_id {
            continue;
        }
        if kept < keep {
            kept += 1;
            continue;
        }
        tracing::info!("Removing old deploy directory: {}", path.display());
        let _ = tokio::fs::remove_dir_all(path).await;
    }

    Ok(())
}
