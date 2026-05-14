use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::api::AppState;
use crate::build::orchestrator::BuildOrchestrator;
use crate::build::BuildConfig;
use crate::db::models::{NewDeploy, NewEnvironment};
use crate::deploy::manager::DeployManager;
use crate::deploy::preview::{matches_preview_pattern, sanitize_branch_for_subdomain};

type HmacSha256 = Hmac<Sha256>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/webhooks/{app_id}/github", post(github_webhook))
        .route("/webhooks/{app_id}/gitlab", post(gitlab_webhook))
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GitHubPushEvent {
    r#ref: String,
    after: String,
    deleted: Option<bool>,
    repository: GitHubRepo,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GitHubRepo {
    clone_url: Option<String>,
    ssh_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GitLabPushEvent {
    r#ref: String,
    after: String,
    project: GitLabProject,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GitLabProject {
    git_http_url: Option<String>,
    git_ssh_url: Option<String>,
}

async fn github_webhook(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if event_type != "push" {
        return StatusCode::OK;
    }

    let Ok(Some(app)) = state.db.get_app(&app_id).await else {
        return StatusCode::NOT_FOUND;
    };

    if let Some(ref secret) = app.webhook_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !validate_github_signature(secret, signature, &body) {
            return StatusCode::UNAUTHORIZED;
        }
    }

    let event: GitHubPushEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let branch = match extract_branch_from_ref(&event.r#ref) {
        Some(b) => b.to_string(),
        None => return StatusCode::OK,
    };

    if event.deleted.unwrap_or(false) {
        handle_branch_delete(&state, &app, &branch).await;
        return StatusCode::OK;
    }

    handle_push(&state, &app, &branch, &event.after).await;
    StatusCode::OK
}

async fn gitlab_webhook(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let event_type = headers
        .get("X-Gitlab-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if event_type != "Push Hook" {
        return StatusCode::OK;
    }

    let Ok(Some(app)) = state.db.get_app(&app_id).await else {
        return StatusCode::NOT_FOUND;
    };

    if let Some(ref secret) = app.webhook_secret {
        let token = headers
            .get("X-Gitlab-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !validate_gitlab_token(secret, token) {
            return StatusCode::UNAUTHORIZED;
        }
    }

    let event: GitLabPushEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let branch = match extract_branch_from_ref(&event.r#ref) {
        Some(b) => b.to_string(),
        None => return StatusCode::OK,
    };

    let is_delete = event.after == "0000000000000000000000000000000000000000";
    if is_delete {
        handle_branch_delete(&state, &app, &branch).await;
        return StatusCode::OK;
    }

    handle_push(&state, &app, &branch, &event.after).await;
    StatusCode::OK
}

async fn handle_push(state: &AppState, app: &crate::db::models::App, branch: &str, sha: &str) {
    if branch == app.git_branch {
        trigger_deploy(state, app, branch, sha, "production").await;
    } else if app.preview_enabled && matches_preview_pattern(&app.preview_branch_pattern, branch) {
        trigger_deploy(state, app, branch, sha, "preview").await;
    }
}

async fn trigger_deploy(
    state: &AppState,
    app: &crate::db::models::App,
    branch: &str,
    sha: &str,
    env_type: &str,
) {
    let env = if env_type == "preview" {
        match state.db.get_environment_by_branch(&app.id, branch).await {
            Ok(Some(env)) => env,
            Ok(None) => {
                let sanitized = sanitize_branch_for_subdomain(branch);
                match state
                    .db
                    .create_environment(&NewEnvironment {
                        app_id: app.id.clone(),
                        name: sanitized,
                        env_type: "preview".to_string(),
                        branch: Some(branch.to_string()),
                    })
                    .await
                {
                    Ok(env) => env,
                    Err(e) => {
                        tracing::error!("Failed to create preview environment: {e}");
                        return;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to look up environment: {e}");
                return;
            }
        }
    } else {
        let Ok(envs) = state.db.list_environments(&app.id).await else {
            return;
        };
        match envs.into_iter().find(|e| e.env_type == "production") {
            Some(env) => env,
            None => match envs_first_or_create(state, app).await {
                Some(env) => env,
                None => return,
            },
        }
    };

    let deploy = match state
        .db
        .create_deploy(&NewDeploy {
            app_id: app.id.clone(),
            environment_id: env.id.clone(),
            git_sha: Some(sha.to_string()),
            server_id: None,
            tag: None,
            no_cache: false,
        })
        .await
    {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to create deploy: {e}");
            return;
        }
    };

    let build_config: Option<BuildConfig> = app
        .build_config
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let lock = state.build_locks.acquire(&app.id).await;
    let app = app.clone();
    let deploy_id = deploy.id.clone();
    let env_clone = env.clone();
    let state = state.clone();

    tokio::spawn(async move {
        let _guard = lock.lock().await;

        let orchestrator =
            BuildOrchestrator::new(state.docker.clone(), state.db.clone(), state.config.clone());

        match orchestrator
            .build(&deploy_id, &app, build_config, false)
            .await
        {
            Ok(result) => {
                let manager = DeployManager::new(
                    state.docker.clone(),
                    state.caddy.clone(),
                    state.db.clone(),
                    state.config.clone(),
                    state.event_bus.clone(),
                    Some(state.agent_registry.clone()),
                );

                let current_deploy = match state.db.get_deploy(&deploy_id).await {
                    Ok(Some(d)) => d,
                    other => {
                        tracing::error!("Failed to re-fetch deploy {deploy_id}: {other:?}");
                        return;
                    }
                };

                if let Err(e) = manager
                    .deploy(&current_deploy, &app, &env_clone, &result.image_ref)
                    .await
                {
                    tracing::error!("Deploy failed for {deploy_id}: {e}");
                }
            }
            Err(e) => {
                tracing::error!("Build failed for {deploy_id}: {e}");
            }
        }
    });
}

async fn envs_first_or_create(
    state: &AppState,
    app: &crate::db::models::App,
) -> Option<crate::db::models::Environment> {
    let envs = state.db.list_environments(&app.id).await.ok()?;
    if let Some(env) = envs.first() {
        return Some(env.clone());
    }
    state
        .db
        .create_environment(&NewEnvironment {
            app_id: app.id.clone(),
            name: "production".to_string(),
            env_type: "production".to_string(),
            branch: None,
        })
        .await
        .ok()
}

async fn handle_branch_delete(state: &AppState, app: &crate::db::models::App, branch: &str) {
    let env = match state.db.get_environment_by_branch(&app.id, branch).await {
        Ok(Some(env)) if env.env_type == "preview" => env,
        _ => return,
    };

    let manager = DeployManager::new(
        state.docker.clone(),
        state.caddy.clone(),
        state.db.clone(),
        state.config.clone(),
        state.event_bus.clone(),
        Some(state.agent_registry.clone()),
    );

    if let Err(e) = manager.teardown(app, &env, "").await {
        tracing::error!("Failed to tear down preview environment: {e}");
    }

    let _ = state.db.delete_env_vars_by_environment(&env.id).await;
    let _ = state.db.delete_environment(&env.id).await;
}

pub fn validate_github_signature(secret: &str, signature_header: &str, body: &[u8]) -> bool {
    let expected_hex = signature_header
        .strip_prefix("sha256=")
        .unwrap_or(signature_header);

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    let result = mac.finalize().into_bytes();
    let computed = hex::encode(result);

    constant_time_eq(computed.as_bytes(), expected_hex.as_bytes())
}

pub fn validate_gitlab_token(secret: &str, token: &str) -> bool {
    constant_time_eq(secret.as_bytes(), token.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

pub fn extract_branch_from_ref(git_ref: &str) -> Option<&str> {
    git_ref.strip_prefix("refs/heads/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_hmac_valid() {
        let secret = "my-secret";
        let body = b"payload body";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(validate_github_signature(secret, &signature, body));
    }

    #[test]
    fn github_hmac_invalid() {
        assert!(!validate_github_signature("secret", "sha256=0000", b"body"));
    }

    #[test]
    fn gitlab_token_valid() {
        assert!(validate_gitlab_token("my-token", "my-token"));
    }

    #[test]
    fn gitlab_token_invalid() {
        assert!(!validate_gitlab_token("my-token", "wrong-token"));
    }

    #[test]
    fn extract_branch() {
        assert_eq!(extract_branch_from_ref("refs/heads/main"), Some("main"));
        assert_eq!(
            extract_branch_from_ref("refs/heads/feature/auth"),
            Some("feature/auth")
        );
        assert_eq!(extract_branch_from_ref("refs/tags/v1.0"), None);
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
    }
}
