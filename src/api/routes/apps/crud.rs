use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::db::models::{NewApp, NewEnvironment, UpdateApp, CONTROL_PLANE_SERVER_ID};

#[derive(Deserialize, Default)]
pub(super) struct ListAppsQuery {
    tag: Option<String>,
    project_id: Option<String>,
}

/// Recognised app deploy modes: `auto` (managed pick), `compose` (managed
/// per-service), `raw-compose` (IF-173, file to CLI). Unset defaults to `auto`.
const VALID_DEPLOY_MODES: &[&str] = &["auto", "compose", "raw-compose"];

/// Validate that an app name is safe to embed in a Docker image reference
/// (`icefall/{name}:{tag}`). Bad names (spaces, colons, uppercase) otherwise
/// surface much later as a cryptic "invalid reference format" build failure.
fn validate_app_name(name: &str) -> Result<(), ApiError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ApiError::BadRequest("App name must not be empty".into()));
    }
    if trimmed.len() > 63 {
        return Err(ApiError::BadRequest(
            "App name must be 63 characters or fewer".into(),
        ));
    }
    let valid = trimmed
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.'));
    let edges_ok = trimmed
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && trimmed
            .bytes()
            .next_back()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if !valid || !edges_ok {
        return Err(ApiError::BadRequest(
            "App name may use only lowercase letters, digits, '-', '_', '.', \
             and must start and end with a letter or digit"
                .into(),
        ));
    }
    Ok(())
}

/// Build a build_config JSON string from the create request's overrides,
/// omitting unset/blank fields. Returns None when no override was provided.
fn build_config_json(body: &CreateAppRequest) -> Option<String> {
    let mut obj = serde_json::Map::new();
    let mut put_str = |key: &str, val: &Option<String>| {
        if let Some(v) = val.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            obj.insert(key.to_string(), serde_json::Value::from(v));
        }
    };
    put_str("build_command", &body.build_command);
    put_str("output_dir", &body.output_dir);
    put_str("start_command", &body.start_command);
    if let Some(port) = body.port {
        obj.insert("port".to_string(), serde_json::Value::from(port));
    }
    if obj.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(obj).to_string())
    }
}

fn validate_deploy_mode(mode: Option<&str>) -> Result<(), ApiError> {
    match mode {
        None => Ok(()),
        Some(m) if VALID_DEPLOY_MODES.contains(&m) => Ok(()),
        Some(m) => Err(ApiError::BadRequest(format!(
            "Invalid deploy_mode '{m}'. Valid: {}",
            VALID_DEPLOY_MODES.join(", ")
        ))),
    }
}

#[derive(Deserialize)]
pub(super) struct CreateAppRequest {
    name: String,
    git_repo: Option<String>,
    git_branch: Option<String>,
    framework: Option<String>,
    image_ref: Option<String>,
    compose_content: Option<String>,
    port: Option<u16>,
    deploy_mode: Option<String>,
    server_id: Option<String>,
    // Build overrides from the create wizard (AC4). Persisted into build_config;
    // deploy-time detection still fills any left unset.
    build_command: Option<String>,
    output_dir: Option<String>,
    start_command: Option<String>,
    base_directory: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct UpdateAppRequest {
    name: Option<String>,
    git_repo: Option<String>,
    git_branch: Option<String>,
    framework: Option<String>,
    build_config: Option<String>,
    resource_limits: Option<String>,
    preview_enabled: Option<bool>,
    preview_branch_pattern: Option<Option<String>>,
    tags: Option<String>,
    volumes: Option<String>,
    image_ref: Option<Option<String>>,
    compose_content: Option<Option<String>>,
    project_id: Option<Option<String>>,
    deploy_mode: Option<String>,
    base_directory: Option<Option<String>>,
    disable_build_cache: Option<bool>,
    git_submodules_enabled: Option<bool>,
    git_lfs_enabled: Option<bool>,
    git_shallow_clone: Option<bool>,
    basic_auth_enabled: Option<bool>,
    basic_auth_username: Option<Option<String>>,
    basic_auth_password: Option<String>,
    pre_deploy_commands: Option<Option<String>>,
    post_deploy_commands: Option<Option<String>>,
    ssh_key_id: Option<Option<String>>,
    ghost_mode_enabled: Option<bool>,
    ghost_mode_idle_minutes: Option<i32>,
    canary_enabled: Option<bool>,
    canary_config: Option<Option<String>>,
    log_noise_patterns: Option<Option<String>>,
    log_highlight_patterns: Option<Option<String>>,
    require_deploy_approval: Option<bool>,
    project_environment_id: Option<Option<String>>,
    github_installation_id: Option<Option<String>>,
}

pub(super) async fn list_apps(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Query(query): Query<ListAppsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Scope the listing to the caller's team, then apply the
    // existing project/tag filters in memory.
    let mut apps = state.db.list_apps_by_team(&ctx.team_id).await?;

    if let Some(pid) = &query.project_id {
        apps.retain(|app| app.project_id.as_deref() == Some(pid.as_str()));
    }

    if let Some(tag) = &query.tag {
        let tag = tag.trim().to_lowercase();
        if !tag.is_empty() {
            apps.retain(|app| {
                app.tags
                    .as_deref()
                    .unwrap_or("")
                    .split(',')
                    .any(|t| t.trim() == tag)
            });
        }
    }

    Ok(Json(serde_json::json!({ "data": apps })))
}

pub(super) async fn create_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Json(body): Json<CreateAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Creating an app requires at least member role in the team.
    ctx.verify_team_access(&ctx.team_id, TeamRole::Member)?;

    validate_app_name(&body.name)?;

    validate_deploy_mode(body.deploy_mode.as_deref())?;

    if let Some(ref yaml) = body.compose_content {
        // Raw Compose (IF-173) hands the file straight to the compose CLI, so
        // only sanity-check that it's YAML; the CLI surfaces schema errors later.
        let is_raw = body.deploy_mode.as_deref() == Some("raw-compose");
        if is_raw {
            if serde_yaml::from_str::<serde_yaml::Value>(yaml).is_err() {
                return Err(ApiError::BadRequest("Invalid YAML".to_string()));
            }
        } else if crate::deploy::compose::ComposeDeployer::parse(yaml).is_err() {
            return Err(ApiError::BadRequest(
                "Invalid Docker Compose YAML".to_string(),
            ));
        }
    }

    let resolved_server_id = if let Some(ref sid) = body.server_id {
        if sid != CONTROL_PLANE_SERVER_ID {
            let server = state
                .db
                .get_server(sid)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("Server {sid} not found")))?;
            if server.status == "offline" || server.status == "enrolling" {
                return Err(ApiError::BadRequest(format!(
                    "Server '{}' is not connected (status: {})",
                    server.name, server.status
                )));
            }
            if server.role == "draining" {
                return Err(ApiError::BadRequest(format!(
                    "Server '{}' is draining and cannot accept new apps",
                    server.name
                )));
            }
        }
        Some(sid.clone())
    } else {
        None
    };

    // Capture build overrides before `body` is partially moved into NewApp.
    let build_config = build_config_json(&body);
    let base_directory = body
        .base_directory
        .as_ref()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());

    let app = state
        .db
        .create_app(&NewApp {
            name: body.name.clone(),
            team_id: ctx.team_id.clone(),
            git_repo: body.git_repo,
            git_branch: body.git_branch.unwrap_or_else(|| "main".into()),
            framework: body.framework,
            image_ref: body.image_ref,
            compose_content: body.compose_content,
            deploy_mode: body.deploy_mode,
            server_id: resolved_server_id,
        })
        .await?;

    if build_config.is_some() || base_directory.is_some() {
        let _ = state
            .db
            .update_app(
                &app.id,
                &UpdateApp {
                    build_config,
                    base_directory: base_directory.map(Some),
                    ..Default::default()
                },
            )
            .await;
    }

    let _ = state
        .db
        .create_environment(&NewEnvironment {
            app_id: app.id.clone(),
            name: "production".into(),
            env_type: "production".into(),
            branch: None,
        })
        .await?;

    Ok(Json(serde_json::json!({ "data": app })))
}

pub(super) async fn get_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Only return the app if it belongs to the caller's team.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    Ok(Json(serde_json::json!({ "data": app })))
}

pub(super) async fn update_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    Json(body): Json<UpdateAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    validate_deploy_mode(body.deploy_mode.as_deref())?;
    if let Some(ref name) = body.name {
        validate_app_name(name)?;
    }

    let app = state
        .db
        .update_app(
            &id,
            &UpdateApp {
                name: body.name,
                git_repo: body.git_repo,
                git_branch: body.git_branch,
                framework: body.framework,
                build_config: body.build_config,
                resource_limits: body.resource_limits,
                preview_enabled: body.preview_enabled,
                preview_branch_pattern: body.preview_branch_pattern,
                tags: body.tags.map(|t| {
                    t.split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(",")
                }),
                volumes: body.volumes,
                image_ref: body.image_ref,
                compose_content: body.compose_content,
                project_id: body.project_id,
                deploy_mode: body.deploy_mode,
                server_id: None,
                base_directory: body.base_directory,
                disable_build_cache: body.disable_build_cache,
                git_submodules_enabled: body.git_submodules_enabled,
                git_lfs_enabled: body.git_lfs_enabled,
                git_shallow_clone: body.git_shallow_clone,
                basic_auth_enabled: body.basic_auth_enabled,
                basic_auth_username: body.basic_auth_username,
                basic_auth_password_hash: body
                    .basic_auth_password
                    .map(|pw| Some(bcrypt::hash(pw, bcrypt::DEFAULT_COST).unwrap_or_default())),
                pre_deploy_commands: body.pre_deploy_commands,
                post_deploy_commands: body.post_deploy_commands,
                ssh_key_id: body.ssh_key_id,
                ghost_mode_enabled: body.ghost_mode_enabled,
                ghost_mode_idle_minutes: body.ghost_mode_idle_minutes,
                canary_enabled: body.canary_enabled,
                canary_config: body.canary_config,
                log_noise_patterns: body.log_noise_patterns,
                log_highlight_patterns: body.log_highlight_patterns,
                require_deploy_approval: body.require_deploy_approval,
                project_environment_id: body.project_environment_id,
                github_installation_id: body.github_installation_id,
                desired_instances: None,
                lb_policy: None,
                lb_health_check_path: None,
                lb_sticky_sessions: None,
            },
        )
        .await?;

    // IF-174: once an app has a GitHub installation and repo, auto-create its
    // webhook. Best-effort, only when not already provisioned — never fail over it.
    if app.github_installation_id.is_some()
        && app.git_repo.is_some()
        && app.webhook_secret.is_none()
    {
        match crate::github::webhook_setup::provision_webhook(&state, &app).await {
            Ok(hook_id) => {
                tracing::info!(app_id = %app.id, hook_id, "auto-created GitHub webhook")
            }
            Err(e) => {
                tracing::warn!(app_id = %app.id, error = %e, "failed to auto-create GitHub webhook")
            }
        }
    }

    Ok(Json(serde_json::json!({ "data": app })))
}

pub(super) async fn delete_app(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Destructive delete — app must belong to the caller's team, admin role.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Admin)?;

    // Raw compose stacks aren't labelled `icefall.app`, so tear them down via
    // `compose down` first (best-effort) to avoid orphaning containers/networks.
    if app.deploy_mode == "raw-compose" {
        let deployer = crate::deploy::raw_compose::RawComposeDeployer::new(
            state.db.clone(),
            state.event_bus.clone(),
            state.config.clone(),
        );
        if let Err(e) = deployer.down(&app).await {
            tracing::warn!("raw compose down failed during delete of {id}: {e}");
        }
    }

    state.db.delete_app(&id).await?;
    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_mode_validation() {
        assert!(validate_deploy_mode(None).is_ok());
        assert!(validate_deploy_mode(Some("auto")).is_ok());
        assert!(validate_deploy_mode(Some("compose")).is_ok());
        assert!(validate_deploy_mode(Some("raw-compose")).is_ok());
        assert!(validate_deploy_mode(Some("bogus")).is_err());
    }

    #[test]
    fn app_name_validation() {
        assert!(validate_app_name("webgl-portfolio").is_ok());
        assert!(validate_app_name("my_app.2").is_ok());
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("My App").is_err()); // space + uppercase
        assert!(validate_app_name("bad:name").is_err()); // colon
        assert!(validate_app_name("-leading").is_err()); // bad edge
        assert!(validate_app_name("npm: not found").is_err());
    }
}
