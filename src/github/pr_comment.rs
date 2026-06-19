//! Preview-environment PR comments (IF-174): post or edit a single comment on the
//! matching PR with the preview URL and status. Best-effort; never blocks a deploy.

use crate::api::AppState;
use crate::db::models::{App, Environment};
use crate::deploy::preview::sanitize_branch_for_subdomain;
use crate::github::client::GitHubClient;
use crate::github::token::get_valid_installation_token;

/// Post or update the preview-env comment for `app`'s `env`. Passing `sha` matches
/// fork PRs too. No-op for non-preview envs, unlinked apps, or PR-less branches.
pub async fn post_preview_comment(
    state: &AppState,
    app: &App,
    env: &Environment,
    status: &str,
    sha: Option<&str>,
) {
    if env.env_type != "preview" {
        return;
    }
    let Some(installation_id) = app.github_installation_id.as_deref() else {
        return;
    };
    let Some(git_repo) = app.git_repo.as_deref() else {
        return;
    };
    let Some(branch) = env.branch.as_deref() else {
        return;
    };
    let Some((owner, repo)) = crate::github::owner_repo(git_repo) else {
        return;
    };

    let resolved = match get_valid_installation_token(&state.db, installation_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(app_id = %app.id, error = %e, "no GitHub token for PR comment");
            return;
        }
    };

    let client = GitHubClient::new(&resolved.api_url);

    // Prefer the commit-based lookup (matches fork PRs); fall back to the
    // branch lookup when no SHA is available (e.g. branch-delete teardown).
    let pr_lookup = match sha {
        Some(sha) => {
            client
                .find_pr_for_commit(&resolved.token, &owner, &repo, sha)
                .await
        }
        None => {
            client
                .find_pr_for_branch(&resolved.token, &owner, &repo, branch)
                .await
        }
    };
    let pr_number = match pr_lookup {
        Ok(Some(n)) => n,
        Ok(None) => return, // No open PR — nothing to comment on.
        Err(e) => {
            tracing::warn!(app_id = %app.id, error = %e, "failed to find PR for preview");
            return;
        }
    };

    let body = comment_body(state, app, branch, status);

    // Edit the existing comment if we've posted one for this (app, PR) before.
    let existing = state
        .db
        .get_github_pr_comment(&app.id, pr_number)
        .await
        .ok()
        .flatten();

    match existing {
        Some(tracked) => {
            if let Err(e) = client
                .update_comment(&resolved.token, &owner, &repo, tracked.comment_id, &body)
                .await
            {
                tracing::warn!(app_id = %app.id, error = %e, "failed to update PR comment");
            }
        }
        None => {
            match client
                .create_comment(&resolved.token, &owner, &repo, pr_number, &body)
                .await
            {
                Ok(comment_id) => {
                    let _ = state
                        .db
                        .upsert_github_pr_comment(
                            &app.id,
                            resolved.installation_id,
                            &format!("{owner}/{repo}"),
                            pr_number,
                            comment_id,
                        )
                        .await;
                }
                Err(e) => {
                    tracing::warn!(app_id = %app.id, error = %e, "failed to create PR comment");
                }
            }
        }
    }
}

/// The preview URL for a branch deploy, if a public domain is configured.
fn preview_url(state: &AppState, app: &App, branch: &str) -> Option<String> {
    state.config.base_domain.as_ref().map(|base| {
        let sanitized = sanitize_branch_for_subdomain(branch);
        format!("https://{sanitized}--{}.{base}", app.name)
    })
}

fn comment_body(state: &AppState, app: &App, branch: &str, status: &str) -> String {
    if status == "destroyed" {
        return "**Icefall preview environment destroyed.**\n\nThe preview for this branch has been torn down.".to_string();
    }
    let url_line = match preview_url(state, app, branch) {
        Some(url) => format!("**Preview URL:** {url}"),
        None => "_No public domain configured; preview not externally reachable._".to_string(),
    };
    format!(
        "**Icefall preview environment** — {status}\n\n{url_line}\n\nBranch: `{branch}`\n\n_Updated automatically by Icefall on each push._"
    )
}
