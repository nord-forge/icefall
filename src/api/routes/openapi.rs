use axum::routing::get;
use axum::{Json, Router};

use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/openapi.json", get(openapi_spec))
}

async fn openapi_spec() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Icefall API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Self-hosted deployment platform API"
        },
        "paths": {
            "/api/v1/auth/login": { "post": { "summary": "Login", "tags": ["Auth"] } },
            "/api/v1/auth/logout": { "post": { "summary": "Logout", "tags": ["Auth"] } },
            "/api/v1/auth/setup": { "get": { "summary": "Setup status" }, "post": { "summary": "Create admin account", "tags": ["Auth"] } },
            "/api/v1/apps": { "get": { "summary": "List apps", "tags": ["Apps"] }, "post": { "summary": "Create app", "tags": ["Apps"] } },
            "/api/v1/apps/{id}": { "get": { "summary": "Get app" }, "put": { "summary": "Update app" }, "delete": { "summary": "Delete app", "tags": ["Apps"] } },
            "/api/v1/apps/{id}/deploys": { "get": { "summary": "List deploys" }, "post": { "summary": "Trigger deploy (optionally scheduled via scheduled_at)", "tags": ["Deploys"] } },
            "/api/v1/deploys/{deploy_id}/reschedule": { "post": { "summary": "Reschedule a scheduled deploy", "tags": ["Deploys"] } },
            "/api/v1/apps/{id}/env": { "get": { "summary": "List env vars" }, "post": { "summary": "Set env var", "tags": ["Env Vars"] } },
            "/api/v1/apps/{id}/domains": { "get": { "summary": "List domains" }, "post": { "summary": "Add domain", "tags": ["Domains"] } },
            "/api/v1/apps/{id}/health": { "get": { "summary": "Health status" }, "put": { "summary": "Configure health check", "tags": ["Health"] } },
            "/api/v1/apps/{id}/metrics": { "get": { "summary": "Current metrics", "tags": ["Metrics"] } },
            "/api/v1/apps/{id}/logs": { "get": { "summary": "Search logs", "tags": ["Logs"] } },
            "/api/v1/databases": { "get": { "summary": "List databases" }, "post": { "summary": "Create database", "tags": ["Databases"] } },
            "/api/v1/databases/{id}": { "get": { "summary": "Get database" }, "delete": { "summary": "Delete database", "tags": ["Databases"] } },
            "/api/v1/users": { "get": { "summary": "List users (admin)", "tags": ["Users"] } },
            "/api/v1/users/invite": { "post": { "summary": "Invite user (admin)", "tags": ["Users"] } },
            "/api/v1/users/me": { "get": { "summary": "Current user profile", "tags": ["Users"] } },
            "/api/v1/tokens": { "get": { "summary": "List API tokens" }, "post": { "summary": "Create token", "tags": ["Tokens"] } },
            "/api/v1/server/status": { "get": { "summary": "Server status", "tags": ["Server"] } },
            "/api/v1/settings": { "get": { "summary": "Platform settings", "tags": ["Settings"] } },
            "/api/v1/events": { "get": { "summary": "SSE event stream", "tags": ["Events"] } },
            "/api/v1/webhooks/{app_id}/github": { "post": { "summary": "GitHub webhook", "tags": ["Webhooks"] } },
            "/api/v1/webhooks/{app_id}/gitlab": { "post": { "summary": "GitLab webhook", "tags": ["Webhooks"] } },
        }
    }))
}
