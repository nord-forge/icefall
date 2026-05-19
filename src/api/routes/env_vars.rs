use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::db::models::NewEnvVar;

const RESERVED_KEYS: &[&str] = &["PORT", "HOST"];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps/{id}/env", get(list_env_vars).post(set_env_var))
        .route("/apps/{id}/env/import", post(import_dotenv))
        .route("/apps/{id}/env/resolved", get(resolved_env_vars))
        .route("/apps/{id}/env/{var_id}", delete(delete_env_var))
}

#[derive(Debug, Deserialize)]
struct ListParams {
    #[serde(default)]
    reveal: bool,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetEnvVarRequest {
    key: String,
    value: String,
    #[serde(default = "default_scope")]
    scope: String,
}

fn default_scope() -> String {
    "shared".to_string()
}

#[derive(Debug, Deserialize)]
struct ImportEnvRequest {
    content: String,
    #[serde(default = "default_scope")]
    scope: String,
}

#[derive(Debug, Serialize)]
struct EnvVarResponse {
    id: String,
    key: String,
    value: String,
    scope: String,
    created_at: String,
}

async fn list_env_vars(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Read-only — the app must belong to the caller's team (viewer).
    state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let envs = state.db.list_environments(&id).await?;
    let env = envs
        .first()
        .ok_or_else(|| ApiError::NotFound("no environments for app".into()))?;

    let vars = state.db.get_env_vars(&env.id).await?;

    let filtered: Vec<EnvVarResponse> = vars
        .into_iter()
        .filter(|v| params.scope.as_ref().is_none_or(|s| &v.scope == s))
        .map(|v| EnvVarResponse {
            id: v.id,
            key: v.key,
            value: if params.reveal {
                v.value
            } else {
                "••••••••".to_string()
            },
            scope: v.scope,
            created_at: v.created_at,
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": filtered })))
}

async fn set_env_var(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    Json(body): Json<SetEnvVarRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    validate_key(&body.key)?;
    validate_scope(&body.scope)?;

    let envs = state.db.list_environments(&id).await?;
    let env = envs
        .first()
        .ok_or_else(|| ApiError::NotFound("no environments for app".into()))?;

    let var = state
        .db
        .set_env_var(&NewEnvVar {
            environment_id: env.id.clone(),
            key: body.key,
            value: body.value,
            scope: body.scope,
        })
        .await?;

    restart_app_containers(&state, &id).await;

    Ok(Json(serde_json::json!({ "data": {
        "id": var.id,
        "key": var.key,
        "scope": var.scope,
    }})))
}

async fn delete_env_var(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path((id, var_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The parent app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    // The env var must actually belong to one of this app's environments —
    // otherwise a caller could delete another app's var.
    let envs = state.db.list_environments(&id).await?;
    let mut belongs = false;
    for env in &envs {
        if state
            .db
            .get_env_vars(&env.id)
            .await?
            .iter()
            .any(|v| v.id == var_id)
        {
            belongs = true;
            break;
        }
    }
    if !belongs {
        return Err(ApiError::NotFound(format!("env var '{var_id}' not found")));
    }

    state.db.delete_env_var(&var_id).await?;
    restart_app_containers(&state, &id).await;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn import_dotenv(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    Json(body): Json<ImportEnvRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    validate_scope(&body.scope)?;

    let envs = state.db.list_environments(&id).await?;
    let env = envs
        .first()
        .ok_or_else(|| ApiError::NotFound("no environments for app".into()))?;

    let pairs = parse_dotenv(&body.content);
    let mut imported = 0;
    let mut skipped = Vec::new();

    for (key, value) in pairs {
        if RESERVED_KEYS.contains(&key.as_str()) {
            skipped.push(key);
            continue;
        }
        if validate_key_str(&key).is_err() {
            skipped.push(key);
            continue;
        }
        state
            .db
            .set_env_var(&NewEnvVar {
                environment_id: env.id.clone(),
                key,
                value,
                scope: body.scope.clone(),
            })
            .await?;
        imported += 1;
    }

    restart_app_containers(&state, &id).await;

    Ok(Json(serde_json::json!({
        "imported": imported,
        "skipped": skipped,
    })))
}

async fn resolved_env_vars(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Read-only — the app must belong to the caller's team (viewer).
    state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let envs = state.db.list_environments(&id).await?;
    let env = envs
        .first()
        .ok_or_else(|| ApiError::NotFound("no environments for app".into()))?;

    let vars = state.db.get_env_vars(&env.id).await?;

    let mut merged = std::collections::HashMap::new();
    for var in &vars {
        if var.scope == "shared" {
            merged.insert(var.key.clone(), var.value.clone());
        }
    }
    for var in &vars {
        if var.scope != "shared" {
            merged.insert(var.key.clone(), var.value.clone());
        }
    }

    merged
        .entry("PORT".to_string())
        .or_insert_with(|| "3000".to_string());
    merged
        .entry("HOST".to_string())
        .or_insert_with(|| "0.0.0.0".to_string());

    let resolved: Vec<serde_json::Value> = merged
        .into_iter()
        .map(|(k, v)| serde_json::json!({"key": k, "value": v}))
        .collect();

    Ok(Json(serde_json::json!({ "data": resolved })))
}

fn validate_key(key: &str) -> Result<(), ApiError> {
    validate_key_str(key)
}

fn validate_key_str(key: &str) -> Result<(), ApiError> {
    if RESERVED_KEYS.contains(&key) {
        return Err(ApiError::BadRequest(format!(
            "{key} is a reserved environment variable"
        )));
    }
    if key.is_empty() {
        return Err(ApiError::BadRequest("key cannot be empty".into()));
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(ApiError::BadRequest(
            "key must contain only uppercase letters, digits, and underscores".into(),
        ));
    }
    Ok(())
}

fn validate_scope(scope: &str) -> Result<(), ApiError> {
    match scope {
        "shared" | "production" | "preview" => Ok(()),
        _ => Err(ApiError::BadRequest(format!(
            "invalid scope '{scope}': must be shared, production, or preview"
        ))),
    }
}

pub fn parse_dotenv(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim().to_string();
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            Some((key, value))
        })
        .collect()
}

async fn restart_app_containers(state: &AppState, app_id: &str) {
    let label = format!("icefall.app={app_id}");
    if let Ok(containers) = state.docker.list_containers(Some(&label)).await {
        for container in containers {
            if container.state == "running" {
                let _ = state.docker.restart_container(&container.id).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dotenv_basic() {
        let input = "FOO=bar\nBAZ=qux";
        let result = parse_dotenv(input);
        assert_eq!(
            result,
            vec![("FOO".into(), "bar".into()), ("BAZ".into(), "qux".into())]
        );
    }

    #[test]
    fn parse_dotenv_comments_and_empty() {
        let input = "# comment\n\nFOO=bar\n  # another comment\nBAZ=qux\n";
        let result = parse_dotenv(input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_dotenv_quoted_values() {
        let input = r#"FOO="hello world"
BAR='single quoted'
BAZ=unquoted"#;
        let result = parse_dotenv(input);
        assert_eq!(result[0], ("FOO".into(), "hello world".into()));
        assert_eq!(result[1], ("BAR".into(), "single quoted".into()));
        assert_eq!(result[2], ("BAZ".into(), "unquoted".into()));
    }

    #[test]
    fn parse_dotenv_equals_in_value() {
        let input = "DATABASE_URL=postgres://user:pass@host/db?sslmode=require";
        let result = parse_dotenv(input);
        assert_eq!(result[0].0, "DATABASE_URL");
        assert_eq!(result[0].1, "postgres://user:pass@host/db?sslmode=require");
    }

    #[test]
    fn reserved_key_rejected() {
        assert!(validate_key("PORT").is_err());
        assert!(validate_key("HOST").is_err());
        assert!(validate_key("MY_VAR").is_ok());
    }

    #[test]
    fn invalid_key_format_rejected() {
        assert!(validate_key("lowercase").is_err());
        assert!(validate_key("has spaces").is_err());
        assert!(validate_key("").is_err());
    }

    #[test]
    fn valid_scope_accepted() {
        assert!(validate_scope("shared").is_ok());
        assert!(validate_scope("production").is_ok());
        assert!(validate_scope("preview").is_ok());
        assert!(validate_scope("invalid").is_err());
    }
}
