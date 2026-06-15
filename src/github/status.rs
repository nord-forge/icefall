//! Deploy commit-status reporting (IF-174): post `icefall/deploy` commit
//! statuses to GitHub as a deploy moves through pending → success/failure.
//!
//! All functions are best-effort: a GitHub failure (no installation, token
//! error, API error) is logged and swallowed so it never blocks a deploy.

use crate::api::AppState;
use crate::db::models::App;
use crate::github::client::GitHubClient;
use crate::github::token::get_valid_installation_token;

/// GitHub commit-status context shown on the PR/commit.
const STATUS_CONTEXT: &str = "icefall/deploy";

/// Post a commit status for `app` at `sha`. `state` is pending/success/failure.
/// No-op (with a debug log) when the app isn't linked to a GitHub installation
/// or lacks a parseable repo — i.e. manual-webhook apps are unaffected.
pub async fn report_deploy_status(
    state: &AppState,
    app: &App,
    sha: &str,
    status: &str,
    description: &str,
) {
    let Some(installation_id) = app.github_installation_id.as_deref() else {
        return;
    };
    let Some(git_repo) = app.git_repo.as_deref() else {
        return;
    };
    let Some((owner, repo)) = owner_repo(git_repo) else {
        tracing::debug!(app_id = %app.id, "cannot parse owner/repo for commit status");
        return;
    };

    let resolved = match get_valid_installation_token(&state.db, installation_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(app_id = %app.id, error = %e, "no GitHub token for commit status");
            return;
        }
    };

    let target_url = app_deploy_url(state, &app.id);
    let client = GitHubClient::new(&resolved.api_url);
    if let Err(e) = client
        .create_commit_status(
            &resolved.token,
            &owner,
            &repo,
            sha,
            status,
            description,
            STATUS_CONTEXT,
            target_url.as_deref(),
        )
        .await
    {
        tracing::warn!(app_id = %app.id, sha, status, error = %e, "failed to post commit status");
    }
}

fn owner_repo(git_repo: &str) -> Option<(String, String)> {
    let trimmed = git_repo
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let normalized = trimmed.replace(':', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        Some((
            parts[parts.len() - 2].to_string(),
            parts[parts.len() - 1].to_string(),
        ))
    } else {
        None
    }
}

/// Link back to the app's deploys view, if a public domain is configured.
fn app_deploy_url(state: &AppState, app_id: &str) -> Option<String> {
    state
        .config
        .base_domain
        .as_ref()
        .map(|domain| format!("https://{domain}/apps/{app_id}"))
}

#[cfg(test)]
mod tests {
    use super::owner_repo;

    #[test]
    fn parses_https_and_ssh() {
        assert_eq!(
            owner_repo("https://github.com/acme/widget.git"),
            Some(("acme".into(), "widget".into()))
        );
        assert_eq!(
            owner_repo("git@github.com:acme/widget"),
            Some(("acme".into(), "widget".into()))
        );
    }

    #[test]
    fn rejects_unparseable() {
        assert_eq!(owner_repo("widget"), None);
    }
}
