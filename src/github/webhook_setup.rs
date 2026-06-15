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

    let (owner, repo) = crate::github::owner_repo(git_repo)
        .ok_or_else(|| format!("cannot parse repo from {git_repo}"))?;

    // GitHub must be able to reach the webhook target. Without a public
    // base_domain the URL is localhost — unreachable — so skip rather than
    // create a webhook that can never deliver (S3).
    if state.config.base_domain.is_none() {
        return Err(
            "no base_domain configured; GitHub cannot reach a localhost webhook URL".to_string(),
        );
    }

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
