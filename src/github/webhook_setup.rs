//! Automatic webhook provisioning (IF-174): when an app is connected to a repo
//! through a GitHub App installation, create the repo webhook via the API instead
//! of asking the user to copy a URL and secret by hand.

use crate::api::AppState;
use crate::db::models::{new_id, App};
use crate::github::client::GitHubClient;
use crate::github::token::get_valid_installation_token;

/// Public base URL of this Icefall instance, used as the webhook target host.
fn instance_base_url(state: &AppState) -> String {
    if let Some(ref domain) = state.config.base_domain {
        format!("https://{domain}")
    } else {
        let addr = &state.config.listen_addr;
        let host = if addr == "0.0.0.0" || addr == "::" {
            "localhost"
        } else {
            addr
        };
        format!("http://{}:{}", host, state.config.listen_port)
    }
}

/// Parse "owner/name" from an app's git_repo URL (https or ssh form).
fn owner_repo_from_url(git_repo: &str) -> Option<(String, String)> {
    let trimmed = git_repo
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git");
    // Take the last two path segments — works for https://host/owner/repo and
    // git@host:owner/repo alike.
    let normalized = trimmed.replace(':', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        let owner = parts[parts.len() - 2].to_string();
        let repo = parts[parts.len() - 1].to_string();
        Some((owner, repo))
    } else {
        None
    }
}

/// Create the GitHub webhook for `app` via its linked installation. Generates and
/// stores a fresh webhook secret on the app. Best-effort: returns an error string
/// the caller can log/surface, but callers treat failure as non-fatal (the app is
/// still created; the user can retry).
pub async fn provision_webhook(state: &AppState, app: &App) -> Result<i64, String> {
    let installation_id = app
        .github_installation_id
        .as_deref()
        .ok_or_else(|| "app has no linked GitHub installation".to_string())?;

    let git_repo = app
        .git_repo
        .as_deref()
        .ok_or_else(|| "app has no git_repo".to_string())?;

    let (owner, repo) = owner_repo_from_url(git_repo)
        .ok_or_else(|| format!("cannot parse repo from {git_repo}"))?;

    let resolved = get_valid_installation_token(&state.db, installation_id).await?;

    // Generate and persist a webhook secret so the incoming-webhook receiver can
    // verify deliveries (it fails closed when the secret is missing).
    let secret = new_id();
    state
        .db
        .set_app_webhook_secret(&app.id, &secret)
        .await
        .map_err(|e| e.to_string())?;

    let target = format!(
        "{}/api/v1/webhooks/{}/github",
        instance_base_url(state),
        app.id
    );

    let client = GitHubClient::new(&resolved.api_url);
    client
        .create_webhook(&resolved.token, &owner, &repo, &target, &secret)
        .await
}

#[cfg(test)]
mod tests {
    use super::owner_repo_from_url;

    #[test]
    fn parses_https_url() {
        assert_eq!(
            owner_repo_from_url("https://github.com/acme/widget"),
            Some(("acme".into(), "widget".into()))
        );
    }

    #[test]
    fn parses_https_url_with_git_suffix() {
        assert_eq!(
            owner_repo_from_url("https://github.com/acme/widget.git"),
            Some(("acme".into(), "widget".into()))
        );
    }

    #[test]
    fn parses_ssh_url() {
        assert_eq!(
            owner_repo_from_url("git@github.com:acme/widget.git"),
            Some(("acme".into(), "widget".into()))
        );
    }

    #[test]
    fn rejects_bare_string() {
        assert_eq!(owner_repo_from_url("not-a-url"), None);
    }
}
