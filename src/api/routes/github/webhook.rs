use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use serde::Deserialize;

use crate::api::routes::webhooks::validate_github_signature;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/github/events", post(github_app_events))
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct InstallationEvent {
    action: String,
    installation: InstallationPayload,
    sender: Option<SenderPayload>,
}

#[derive(Debug, Deserialize)]
struct InstallationPayload {
    id: i64,
    account: AccountPayload,
    app_id: i64,
}

#[derive(Debug, Deserialize)]
struct AccountPayload {
    login: String,
    r#type: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SenderPayload {
    login: String,
}

/// Handles incoming webhook events from GitHub Apps.
///
/// This endpoint verifies the webhook signature using the stored webhook secret
/// for the GitHub App, then dispatches based on event type.
async fn github_app_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    tracing::info!(event_type, "Received GitHub App webhook event");

    // For installation events, we need to parse the body to find the app_id
    // so we can look up the webhook secret for signature verification
    match event_type {
        "installation" => handle_installation_event(&state, &headers, &body).await,
        "push" => handle_push_event(&state, &headers, &body).await,
        "pull_request" => handle_pull_request_event(&state, &headers, &body).await,
        "ping" => {
            tracing::info!("GitHub App ping received");
            StatusCode::OK
        }
        _ => {
            tracing::info!(event_type, "Ignoring unhandled GitHub App event type");
            StatusCode::OK
        }
    }
}

/// Verify the webhook signature against all known GitHub Apps.
///
/// Since the webhook URL is shared across all apps for this instance,
/// we try each app's webhook secret until one validates.
async fn verify_signature(state: &AppState, headers: &HeaderMap, body: &[u8]) -> Option<String> {
    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if signature.is_empty() {
        return None;
    }

    let apps = match state.db.list_github_apps().await {
        Ok(apps) => apps,
        Err(e) => {
            tracing::error!("Failed to list GitHub Apps for signature verification: {e}");
            return None;
        }
    };

    for app in &apps {
        if validate_github_signature(&app.webhook_secret, signature, body) {
            return Some(app.id.clone());
        }
    }

    None
}

async fn handle_installation_event(
    state: &AppState,
    headers: &HeaderMap,
    body: &Bytes,
) -> StatusCode {
    // Verify signature against known apps
    if verify_signature(state, headers, body).await.is_none() {
        tracing::warn!("Invalid webhook signature for installation event");
        return StatusCode::UNAUTHORIZED;
    }

    let event: InstallationEvent = match serde_json::from_slice(body) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to parse installation event: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    match event.action.as_str() {
        "created" => {
            tracing::info!(
                installation_id = event.installation.id,
                account = %event.installation.account.login,
                app_id = event.installation.app_id,
                "GitHub App installed"
            );

            // Find which of our stored apps matches this app_id
            let apps = match state.db.list_github_apps().await {
                Ok(apps) => apps,
                Err(e) => {
                    tracing::error!("Failed to list GitHub Apps: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            };

            let matching_app = apps.iter().find(|a| a.app_id == event.installation.app_id);

            // Create the installation record
            match state
                .db
                .create_github_installation(
                    event.installation.id,
                    &event.installation.account.login,
                    &event.installation.account.r#type,
                )
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        installation_id = event.installation.id,
                        "GitHub installation created"
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to create GitHub installation: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            }

            // Link the installation to our GitHub App
            if let Some(app) = matching_app {
                if let Err(e) = state
                    .db
                    .update_github_installation_app_id(event.installation.id, &app.id)
                    .await
                {
                    tracing::error!(
                        "Failed to link installation {} to app {}: {e}",
                        event.installation.id,
                        app.id
                    );
                }
            }

            StatusCode::OK
        }
        "deleted" => {
            tracing::info!(
                installation_id = event.installation.id,
                account = %event.installation.account.login,
                "GitHub App uninstalled"
            );

            // Find and delete the installation by installation_id
            let installations = match state.db.list_github_installations().await {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("Failed to list installations: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            };

            if let Some(inst) = installations
                .iter()
                .find(|i| i.installation_id == event.installation.id)
            {
                if let Err(e) = state.db.delete_github_installation(&inst.id).await {
                    tracing::error!("Failed to delete installation: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            }

            StatusCode::OK
        }
        other => {
            tracing::info!(action = other, "Ignoring installation event action");
            StatusCode::OK
        }
    }
}

async fn handle_push_event(state: &AppState, headers: &HeaderMap, body: &Bytes) -> StatusCode {
    if verify_signature(state, headers, body).await.is_none() {
        tracing::warn!("Invalid webhook signature for push event");
        return StatusCode::UNAUTHORIZED;
    }

    // For push events from GitHub Apps, we delegate to the existing webhook
    // infrastructure. The push payload contains repository info that lets us
    // find the matching app and trigger a deploy.
    let push: serde_json::Value = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to parse push event: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    let repo_url = push
        .get("repository")
        .and_then(|r| r.get("clone_url"))
        .and_then(|u| u.as_str());

    let Some(repo_url) = repo_url else {
        tracing::warn!("Push event missing repository.clone_url");
        return StatusCode::OK;
    };

    // Look up the Icefall app that matches this repo
    match state.db.get_app_by_repo(repo_url).await {
        Ok(Some(app)) => {
            tracing::info!(
                app_id = %app.id,
                app_name = %app.name,
                repo = %repo_url,
                "GitHub App push event matched to app, delegating to webhook handler"
            );

            // Call the handler directly in the same task; the handler itself
            // spawns a background task for the build/deploy.
            crate::api::routes::webhooks::handle_github_push_for_app(state, &app.id, headers, body)
                .await;

            StatusCode::OK
        }
        Ok(None) => {
            tracing::info!(repo = %repo_url, "Push event for unlinked repository, ignoring");
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("Failed to look up app by repo: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn handle_pull_request_event(
    state: &AppState,
    headers: &HeaderMap,
    body: &Bytes,
) -> StatusCode {
    if verify_signature(state, headers, body).await.is_none() {
        tracing::warn!("Invalid webhook signature for pull_request event");
        return StatusCode::UNAUTHORIZED;
    }

    // Pull request events can be used for preview deployments in the future.
    // For now, log and acknowledge.
    tracing::info!("GitHub App pull_request event received");
    StatusCode::OK
}
