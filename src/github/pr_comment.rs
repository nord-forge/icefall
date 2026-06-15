//! Preview-environment PR comments (IF-174). When a preview env deploys, post
//! (or edit) a single comment on the matching PR with the preview URL and
//! status. All functions are best-effort and never block a deploy.

use crate::api::AppState;
use crate::db::models::{App, Environment};
use crate::deploy::preview::sanitize_branch_for_subdomain;
use crate::github::client::GitHubClient;
use crate::github::token::get_valid_installation_token;

/// Post or update the preview-env comment for `app`'s `env`. `status` is a short
/// label (e.g. "success", "destroyed"). No-op for non-preview envs, apps with no
/// linked installation, or branches with no open PR.
pub async fn post_preview_comment(state: &AppState, app: &App, env: &Environment, status: &str) {
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
    let Some((owner, repo)) = owner_repo(git_repo) else {
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

    // Resolve the PR for this branch.
    let pr_number = match client
        .find_pr_for_branch(&resolved.token, &owner, &repo, branch)
        .await
    {
        Ok(Some(n)) => n,
        Ok(None) => return, // No open PR — nothing to comment on.
        Err(e) => {
            tracing::warn!(app_id = %app.id, error = %e, "failed to find PR for branch");
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
                            resolved_installation_numeric(state, installation_id).await,
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

/// The numeric installation id for the (DB) installation id, for comment tracking.
async fn resolved_installation_numeric(state: &AppState, installation_db_id: &str) -> i64 {
    state
        .db
        .get_github_installation(installation_db_id)
        .await
        .ok()
        .flatten()
        .map(|i| i.installation_id)
        .unwrap_or_default()
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
