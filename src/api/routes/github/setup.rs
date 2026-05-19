use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::{new_id, now_iso8601, GitHubApp};
use crate::github::client::GitHubClient;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/github/setup", get(get_manifest))
        .route("/github/callback", get(handle_callback))
        .route("/github/apps", get(list_apps))
        .route("/github/apps/seed-demo", post(seed_demo))
        .route("/github/apps/{id}", delete(delete_app))
}

/// Returns the GitHub App Manifest and form action URL. The frontend POSTs the manifest
/// to `form_action`; GitHub creates the app and redirects back with a `code` parameter.
async fn get_manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let instance_url = build_instance_url(&state);

    // Generate a short random suffix for uniqueness
    let suffix: String = new_id().chars().take(8).collect();
    let app_name = format!("icefall-{suffix}");

    let is_public = has_public_domain(&state);

    let mut manifest = serde_json::json!({
        "name": app_name,
        "url": instance_url,
        "redirect_url": format!("{instance_url}/api/v1/github/callback"),
        "setup_url": format!("{instance_url}/settings"),
        "callback_urls": [
            format!("{instance_url}/api/v1/github/callback")
        ],
        "setup_on_update": true,
        "public": false,
        "default_permissions": {
            "contents": "read",
            "metadata": "read",
            "pull_requests": "write"
        },
        "default_events": [
            "push",
            "pull_request"
        ]
    });

    if is_public {
        manifest["hook_attributes"] = serde_json::json!({
            "url": format!("{instance_url}/api/v1/github/events"),
            "active": true
        });
    }

    Ok(Json(serde_json::json!({
        "form_action": "https://github.com/settings/apps/new",
        "manifest": manifest,
        "webhooks_active": is_public,
        "instance_url": instance_url,
    })))
}

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
}

/// Handles the GitHub callback after manifest-flow app creation: exchanges the code for
/// credentials, stores them encrypted, and redirects the user to install the app.
async fn handle_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<CallbackParams>,
) -> Result<Response, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let github_client = GitHubClient::new("https://api.github.com");

    let app_from_manifest = github_client
        .exchange_manifest_code(&params.code)
        .await
        .map_err(|e| {
            tracing::error!("Failed to exchange manifest code: {e}");
            ApiError::BadRequest(format!("Failed to register GitHub App: {e}"))
        })?;

    tracing::info!(
        app_id = app_from_manifest.id,
        app_name = %app_from_manifest.name,
        "GitHub App created via manifest flow"
    );

    let now = now_iso8601();
    let github_app = GitHubApp {
        id: new_id(),
        name: app_from_manifest.name.clone(),
        app_id: app_from_manifest.id,
        client_id: app_from_manifest.client_id,
        client_secret: app_from_manifest.client_secret,
        private_key: app_from_manifest.pem,
        webhook_secret: app_from_manifest.webhook_secret,
        html_url: app_from_manifest.html_url,
        api_url: "https://api.github.com".to_string(),
        owner_id: user.id,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.create_github_app(&github_app).await?;

    // Redirect user to install the app on their account/org
    let install_url = format!(
        "https://github.com/apps/{}/installations/new",
        app_from_manifest.name
    );

    Ok(Redirect::temporary(&install_url).into_response())
}

/// List all configured GitHub Apps.
async fn list_apps(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let apps = state.db.list_github_apps().await?;

    // Return a safe view without secrets (serde skip_serializing handles that)
    Ok(Json(serde_json::json!({ "data": apps })))
}

/// Delete a GitHub App and unlink its installations.
async fn delete_app(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let app = state
        .db
        .get_github_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("GitHub App {id} not found")))?;

    tracing::info!(
        app_id = app.app_id,
        app_name = %app.name,
        "Deleting GitHub App"
    );

    state.db.delete_github_app(&id).await?;

    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

/// Build the instance URL from config, falling back to a sensible default.
fn build_instance_url(state: &AppState) -> String {
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

fn has_public_domain(state: &AppState) -> bool {
    state.config.base_domain.is_some()
}

async fn seed_demo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let app = GitHubApp {
        id: new_id(),
        name: "icefall-demo".to_string(),
        app_id: 123456,
        client_id: "Iv1.demo000000000000".to_string(),
        client_secret: "demo_client_secret_not_real".to_string(),
        private_key: "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0demo\n-----END RSA PRIVATE KEY-----\n".to_string(),
        webhook_secret: "demo_webhook_secret".to_string(),
        html_url: "https://github.com/apps/icefall-demo".to_string(),
        api_url: "https://api.github.com".to_string(),
        owner_id: user.id.clone(),
        created_at: now_iso8601(),
        updated_at: now_iso8601(),
    };

    let created = state
        .db
        .create_github_app(&app)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create demo app: {e}")))?;

    let installation_id: i64 = 98765432;
    state
        .db
        .create_github_installation(installation_id, "NickBevers", "user")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create demo installation: {e}")))?;

    state
        .db
        .update_github_installation_app_id(installation_id, &created.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to link installation: {e}")))?;

    tracing::info!("Seeded demo GitHub App and installation for UI testing");

    Ok(Json(serde_json::json!({
        "message": "Demo GitHub App and installation created",
        "data": {
            "app_name": created.name,
            "app_id": created.app_id,
            "installation_account": "NickBevers",
        }
    })))
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_instance_url_with_base_domain() {
        let config = crate::config::IcefallConfig {
            base_domain: Some("paas.example.com".to_string()),
            ..Default::default()
        };

        assert_eq!(
            build_instance_url_from_config(&config),
            "https://paas.example.com"
        );
    }

    #[test]
    fn build_instance_url_without_base_domain() {
        let config = crate::config::IcefallConfig::default();
        let expected = format!("http://{}:{}", config.listen_addr, config.listen_port);
        assert_eq!(build_instance_url_from_config(&config), expected);
    }

    fn build_instance_url_from_config(config: &crate::config::IcefallConfig) -> String {
        if let Some(ref domain) = config.base_domain {
            format!("https://{domain}")
        } else {
            format!("http://{}:{}", config.listen_addr, config.listen_port)
        }
    }
}
