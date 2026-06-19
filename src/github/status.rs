//! Deploy commit-status reporting (IF-174): post `icefall/deploy` commit statuses
//! as a deploy moves pending → success/failure. Best-effort; never blocks a deploy.

use crate::api::AppState;
use crate::db::models::App;
use crate::github::client::GitHubClient;
use crate::github::token::get_valid_installation_token;

/// GitHub commit-status context shown on the PR/commit.
const STATUS_CONTEXT: &str = "icefall/deploy";

/// Post a commit status for `app` at `sha` (state pending/success/failure). No-op
/// when the app isn't GitHub-linked or lacks a parseable repo.
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
    let Some((owner, repo)) = crate::github::owner_repo(git_repo) else {
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

/// Report `pending`, then spawn a watcher that polls the deploy's DB status until
/// terminal and reports it. No-op when not GitHub-linked or the deploy has no SHA.
pub fn watch_deploy(state: &AppState, app: &App, deploy_id: &str, sha: Option<&str>) {
    // Manual deploys may have no SHA; without one there's nothing to report on.
    let (Some(_), Some(sha)) = (app.github_installation_id.as_deref(), sha) else {
        return;
    };
    let state = state.clone();
    let app = app.clone();
    let deploy_id = deploy_id.to_string();
    let sha = sha.to_string();

    tokio::spawn(async move {
        report_deploy_status(&state, &app, &sha, "pending", "Deploy started").await;

        // Poll until terminal (≤ ~20 min), then report once.
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));
        for _ in 0..120 {
            ticker.tick().await;
            let Ok(Some(deploy)) = state.db.get_deploy(&deploy_id).await else {
                continue;
            };
            match deploy.status.as_str() {
                "running" => {
                    report_deploy_status(&state, &app, &sha, "success", "Deployed successfully")
                        .await;
                    return;
                }
                "failed" => {
                    report_deploy_status(&state, &app, &sha, "failure", "Deploy failed").await;
                    return;
                }
                "cancelled" => {
                    report_deploy_status(&state, &app, &sha, "error", "Deploy cancelled").await;
                    return;
                }
                _ => {} // still building/deploying — keep waiting
            }
        }
        tracing::warn!(
            deploy_id,
            "deploy status watcher timed out before terminal state"
        );
    });
}

/// Link back to the app's deploys view, if a public domain is configured.
fn app_deploy_url(state: &AppState, app_id: &str) -> Option<String> {
    state
        .config
        .base_domain
        .as_ref()
        .map(|domain| format!("https://{domain}/apps/{app_id}"))
}
