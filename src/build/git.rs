use std::path::{Path, PathBuf};

use crate::build::BuildError;

pub struct GitCloneOptions {
    pub repo_url: String,
    pub branch: Option<String>,
    pub sha: Option<String>,
    pub ssh_key_path: Option<PathBuf>,
    pub token: Option<String>,
    pub submodules: bool,
    pub lfs: bool,
    pub shallow: bool,
}

pub struct CloneResult {
    pub work_dir: PathBuf,
    pub resolved_sha: String,
}

pub async fn clone_repo(
    opts: &GitCloneOptions,
    work_dir: &Path,
) -> Result<CloneResult, BuildError> {
    tokio::fs::create_dir_all(work_dir).await?;

    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("clone");

    if opts.shallow {
        cmd.arg("--depth").arg("1");
    }

    if let Some(ref branch) = opts.branch {
        cmd.arg("--branch").arg(branch);
    }

    let repo_url = if let Some(ref token) = opts.token {
        inject_token_into_url(&opts.repo_url, token)
    } else {
        opts.repo_url.clone()
    };

    if let Some(ref key_path) = opts.ssh_key_path {
        cmd.env(
            "GIT_SSH_COMMAND",
            format!("ssh -i {} -o StrictHostKeyChecking=no", key_path.display()),
        );
    }

    cmd.arg(&repo_url).arg(work_dir);

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::GitClone(stderr.trim().to_string()));
    }

    if opts.submodules {
        let mut sub_cmd = tokio::process::Command::new("git");
        sub_cmd.args(["submodule", "update", "--init", "--recursive"]);
        if opts.shallow {
            sub_cmd.arg("--depth").arg("1");
        }
        sub_cmd.current_dir(work_dir);
        if let Some(ref key_path) = opts.ssh_key_path {
            sub_cmd.env(
                "GIT_SSH_COMMAND",
                format!("ssh -i {} -o StrictHostKeyChecking=no", key_path.display()),
            );
        }
        let output = sub_cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::GitClone(format!(
                "submodule init failed: {}",
                stderr.trim()
            )));
        }
    }

    if opts.lfs {
        let install = tokio::process::Command::new("git")
            .args(["lfs", "install"])
            .current_dir(work_dir)
            .output()
            .await?;
        if !install.status.success() {
            let stderr = String::from_utf8_lossy(&install.stderr);
            return Err(BuildError::GitClone(format!(
                "git lfs install failed: {} (is git-lfs installed?)",
                stderr.trim()
            )));
        }

        let pull = tokio::process::Command::new("git")
            .args(["lfs", "pull"])
            .current_dir(work_dir)
            .output()
            .await?;
        if !pull.status.success() {
            let stderr = String::from_utf8_lossy(&pull.stderr);
            return Err(BuildError::GitClone(format!(
                "git lfs pull failed: {}",
                stderr.trim()
            )));
        }
    }

    if let Some(ref sha) = opts.sha {
        let fetch = tokio::process::Command::new("git")
            .args(["fetch", "--depth", "1", "origin", sha])
            .current_dir(work_dir)
            .output()
            .await?;

        if !fetch.status.success() {
            let stderr = String::from_utf8_lossy(&fetch.stderr);
            return Err(BuildError::GitCheckout(stderr.trim().to_string()));
        }

        let checkout = tokio::process::Command::new("git")
            .args(["checkout", sha])
            .current_dir(work_dir)
            .output()
            .await?;

        if !checkout.status.success() {
            let stderr = String::from_utf8_lossy(&checkout.stderr);
            return Err(BuildError::GitCheckout(stderr.trim().to_string()));
        }
    }

    let resolved_sha = resolve_sha(work_dir).await?;

    Ok(CloneResult {
        work_dir: work_dir.to_path_buf(),
        resolved_sha,
    })
}

async fn resolve_sha(work_dir: &Path) -> Result<String, BuildError> {
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(work_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(BuildError::GitClone(
            "failed to resolve HEAD SHA".to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn inject_token_into_url(url: &str, token: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://") {
        format!("https://x-access-token:{token}@{rest}")
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_local_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        tokio::process::Command::new("git")
            .args(["init", "--bare"])
            .arg(path)
            .output()
            .await
            .unwrap();

        let work = TempDir::new().unwrap();
        tokio::process::Command::new("git")
            .args([
                "clone",
                &path.to_string_lossy(),
                &work.path().to_string_lossy(),
            ])
            .output()
            .await
            .unwrap();

        tokio::fs::write(work.path().join("hello.txt"), "hello world")
            .await
            .unwrap();

        tokio::process::Command::new("git")
            .args(["add", "."])
            .current_dir(work.path())
            .output()
            .await
            .unwrap();

        tokio::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(work.path())
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .await
            .unwrap();

        tokio::process::Command::new("git")
            .args(["push"])
            .current_dir(work.path())
            .output()
            .await
            .unwrap();

        dir
    }

    #[tokio::test]
    async fn clones_local_repo() {
        let bare = create_local_repo().await;
        let dest = TempDir::new().unwrap();
        let work_dir = dest.path().join("cloned");

        let opts = GitCloneOptions {
            repo_url: bare.path().to_string_lossy().to_string(),
            branch: None,
            sha: None,
            ssh_key_path: None,
            token: None,
            submodules: false,
            lfs: false,
            shallow: true,
        };

        let result = clone_repo(&opts, &work_dir).await.unwrap();
        assert!(!result.resolved_sha.is_empty());
        assert!(work_dir.join("hello.txt").exists());
    }

    #[tokio::test]
    async fn fails_on_invalid_repo() {
        let dest = TempDir::new().unwrap();
        let work_dir = dest.path().join("cloned");

        let opts = GitCloneOptions {
            repo_url: "/nonexistent/repo".to_string(),
            branch: None,
            sha: None,
            ssh_key_path: None,
            token: None,
            submodules: false,
            lfs: false,
            shallow: true,
        };

        assert!(clone_repo(&opts, &work_dir).await.is_err());
    }

    #[test]
    fn injects_token_into_https_url() {
        let url = inject_token_into_url("https://github.com/user/repo.git", "ghp_abc123");
        assert_eq!(
            url,
            "https://x-access-token:ghp_abc123@github.com/user/repo.git"
        );
    }

    #[test]
    fn leaves_ssh_url_unchanged() {
        let url = inject_token_into_url("git@github.com:user/repo.git", "token");
        assert_eq!(url, "git@github.com:user/repo.git");
    }
}
