use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/call", post(call_tool))
        .route("/mcp/resources", get(list_resources))
        .route("/mcp/resources/read", post(read_resource))
        .route("/mcp/prompts", get(list_prompts))
        .route("/mcp/prompts/get", post(get_prompt))
}

async fn list_tools() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "tools": [
            // Core app management (existing)
            tool_def("list_apps", "List all deployed applications with their status", &[]),
            tool_def("get_app", "Get detailed information about a specific app", &[param("app_id", "string", "The app ID")]),
            tool_def("deploy_app", "Trigger a new deploy for an app", &[param("app_id", "string", "The app ID"), param("no_cache", "boolean", "Force rebuild without cache (optional)"), param("tag", "string", "Git tag to deploy (optional)")]),
            tool_def("get_deploy_status", "Get current deploy status and recent history", &[param("app_id", "string", "The app ID")]),
            tool_def("get_logs", "Retrieve recent logs for an app", &[param("app_id", "string", "The app ID"), param("search", "string", "Search term (optional)"), param("limit", "number", "Max lines (default 100)")]),
            tool_def("set_env_var", "Set an environment variable for an app", &[param("app_id", "string", "The app ID"), param("key", "string", "Variable name"), param("value", "string", "Variable value"), param("scope", "string", "Scope: shared, production, or preview")]),
            tool_def("get_env_vars", "List environment variables for an app (values masked)", &[param("app_id", "string", "The app ID")]),
            tool_def("create_database", "Provision a managed database", &[param("name", "string", "Database name"), param("db_type", "string", "Type: postgres, mysql, redis, mongo")]),
            tool_def("list_databases", "List all managed databases", &[]),
            tool_def("get_health_status", "Get health check status for an app", &[param("app_id", "string", "The app ID")]),
            tool_def("get_server_status", "Get server resource overview (CPU, memory, disk)", &[]),
            tool_def("add_domain", "Add a custom domain to an app", &[param("app_id", "string", "The app ID"), param("domain", "string", "Domain name")]),
            tool_def("restart_app", "Restart an app's container", &[param("app_id", "string", "The app ID")]),

            // IF-195: Workflow tools
            tool_def("diagnose", "Pull logs, health status, metrics, and recent deploys for an app — structured diagnostic summary", &[param("app_id", "string", "The app ID")]),
            tool_def("suggest_fix", "Given a diagnostic result, suggest actionable next steps", &[param("app_id", "string", "The app ID"), param("issue", "string", "Description of the issue")]),
            tool_def("cancel_deploy", "Cancel an in-progress deployment", &[param("deploy_id", "string", "The deploy ID")]),
            tool_def("rollback_deploy", "Rollback to a previous deploy", &[param("app_id", "string", "The app ID"), param("deploy_id", "string", "The deploy ID to rollback to")]),
            tool_def("approve_deploy", "Approve a pending deployment", &[param("deploy_id", "string", "The deploy ID")]),

            // IF-195: Bulk operations
            tool_def("bulk_restart", "Restart multiple apps by project or tag", &[param("project_id", "string", "Project ID (optional)"), param("tag", "string", "Tag filter (optional)")]),
            tool_def("bulk_deploy", "Deploy all apps in a project or matching a tag", &[param("project_id", "string", "Project ID (optional)"), param("tag", "string", "Tag filter (optional)")]),

            // IF-195: Resource creation
            tool_def("create_app", "Create a new app with full configuration", &[param("name", "string", "App name"), param("git_repo", "string", "Git repo URL"), param("branch", "string", "Branch (default: main)"), param("framework", "string", "Framework hint (optional)")]),

            // IF-195: Server management
            tool_def("list_servers", "List all servers with health and metrics", &[]),
            tool_def("server_forecast", "Get resource usage forecast for a server", &[param("server_id", "string", "The server ID")]),

            // IF-195: Utility
            tool_def("export_bundle", "Export an app as a portable .icefall bundle", &[param("app_id", "string", "The app ID")]),
            tool_def("search", "Search across all resources (apps, databases, servers, domains)", &[param("query", "string", "Search query")]),
            tool_def("get_analytics", "Get deploy analytics (frequency, success rate, build times)", &[param("days", "number", "Time range in days (default 30)")]),
        ]
    }))
}

#[derive(Deserialize)]
struct ToolCallRequest {
    tool: String,
    params: serde_json::Value,
}

async fn call_tool(
    State(state): State<AppState>,
    ctx: crate::api::team_auth::TeamCtx,
    Json(body): Json<ToolCallRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Resolve the caller's active team — resources created or mutated
    // through MCP tools are scoped to it.
    let user = &ctx.user;

    let can_write = user.role == "admin" || user.role == "deployer";
    let write_tools = [
        "deploy_app",
        "set_env_var",
        "create_database",
        "add_domain",
        "restart_app",
        "cancel_deploy",
        "rollback_deploy",
        "approve_deploy",
        "bulk_restart",
        "bulk_deploy",
        "create_app",
    ];

    if !can_write && write_tools.contains(&body.tool.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Your role '{}' cannot use '{}'. Write access requires deployer or admin role.",
            user.role, body.tool
        )));
    }

    let p = &body.params;
    let result = match body.tool.as_str() {
        "list_apps" => {
            let apps = state.db.list_apps().await?;
            serde_json::json!({ "apps": apps })
        }
        "get_app" => {
            let id = str_param(p, "app_id")?;
            let app = state
                .db
                .get_app(&id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("app {id}")))?;
            serde_json::json!({ "app": app })
        }
        "deploy_app" => {
            let id = str_param(p, "app_id")?;
            let app = state
                .db
                .get_app(&id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("app {id}")))?;
            let envs = state.db.list_environments(&id).await?;
            let env = envs
                .first()
                .ok_or_else(|| ApiError::BadRequest("app has no environments".into()))?;
            let no_cache = p.get("no_cache").and_then(|v| v.as_bool()).unwrap_or(false);
            let deploy = state
                .db
                .create_deploy(&crate::db::models::NewDeploy {
                    app_id: id.clone(),
                    environment_id: env.id.clone(),
                    git_sha: None,
                    server_id: None,
                    tag: None,
                    no_cache,
                })
                .await?;
            serde_json::json!({ "deploy_id": deploy.id, "status": deploy.status, "message": format!("Deploy triggered for {}", app.name) })
        }
        "get_deploy_status" => {
            let id = str_param(p, "app_id")?;
            let deploys = state.db.list_deploys(&id, 5).await?;
            serde_json::json!({ "deploys": deploys })
        }
        "get_logs" => {
            let id = str_param(p, "app_id")?;
            let search = p.get("search").and_then(|v| v.as_str());
            let limit = p
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(100) as usize;
            let logs = state.log_store.search(&id, search, None, limit).await;
            let count = logs.len();
            serde_json::json!({ "logs": logs, "count": count })
        }
        "set_env_var" => {
            let id = str_param(p, "app_id")?;
            let key = str_param(p, "key")?;
            let value = str_param(p, "value")?;
            let scope = p.get("scope").and_then(|v| v.as_str()).unwrap_or("shared");
            let envs = state.db.list_environments(&id).await?;
            let env = envs
                .first()
                .ok_or_else(|| ApiError::BadRequest("app has no environments".into()))?;
            state
                .db
                .set_env_var(&crate::db::models::NewEnvVar {
                    environment_id: env.id.clone(),
                    key: key.clone(),
                    value,
                    scope: scope.to_string(),
                })
                .await?;
            serde_json::json!({ "message": format!("Set {key} for app {id}") })
        }
        "get_env_vars" => {
            let id = str_param(p, "app_id")?;
            let envs = state.db.list_environments(&id).await?;
            let env = envs
                .first()
                .ok_or_else(|| ApiError::BadRequest("app has no environments".into()))?;
            let vars = state.db.get_env_vars(&env.id).await?;
            let masked: Vec<serde_json::Value> = vars
                .iter()
                .map(|v| serde_json::json!({ "key": v.key, "scope": v.scope, "value": "••••••••" }))
                .collect();
            serde_json::json!({ "env_vars": masked })
        }
        "create_database" => {
            let name = str_param(p, "name")?;
            let db_type = str_param(p, "db_type")?;
            let db = state
                .db
                .create_managed_db(&crate::db::models::NewManagedDatabase {
                    name: name.clone(),
                    db_type: db_type.clone(),
                    app_id: None,
                    // The database belongs to the caller's active team.
                    team_id: ctx.team_id.clone(),
                })
                .await?;
            serde_json::json!({ "database_id": db.id, "name": db.name, "type": db.db_type, "message": format!("Created {db_type} database '{name}'") })
        }
        "list_databases" => {
            let dbs = state.db.list_managed_dbs().await?;
            serde_json::json!({ "databases": dbs })
        }
        "get_health_status" => {
            let id = str_param(p, "app_id")?;
            let checks = state.db.get_health_checks(&id).await?;
            let check_ids: Vec<String> = checks.iter().map(|c| c.id.clone()).collect();
            let all_events = state
                .db
                .get_health_events_for_checks(&check_ids, 10)
                .await?;
            let mut results = Vec::with_capacity(checks.len());
            for check in &checks {
                let events: Vec<_> = all_events
                    .iter()
                    .filter(|e| e.health_check_id == check.id)
                    .collect();
                let status = events.first().map_or("unknown", |e| e.status.as_str());
                results.push(serde_json::json!({ "check_type": check.check_type, "status": status, "recent_events": events.len() }));
            }
            serde_json::json!({ "health_checks": results })
        }
        "get_server_status" => {
            let metrics = state.server_metrics.read().await;
            serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "cpu_percent": metrics.cpu_percent,
                "memory_used_gb": metrics.memory_used_bytes as f64 / 1e9,
                "memory_total_gb": metrics.memory_total_bytes as f64 / 1e9,
                "disk_used_gb": metrics.disk_used_bytes as f64 / 1e9,
                "disk_total_gb": metrics.disk_total_bytes as f64 / 1e9,
            })
        }
        "add_domain" => {
            let id = str_param(p, "app_id")?;
            let domain = str_param(p, "domain")?;
            let d = state
                .db
                .add_domain(&crate::db::models::NewDomain {
                    app_id: id,
                    domain: domain.clone(),
                    path: None,
                })
                .await?;
            serde_json::json!({ "domain_id": d.id, "domain": d.domain, "message": format!("Added domain {domain}") })
        }
        "restart_app" => {
            let id = str_param(p, "app_id")?;
            let label = format!("icefall.app={id}");
            let containers = state
                .docker
                .list_containers(Some(&label))
                .await
                .map_err(ApiError::internal)?;
            let mut restarted = 0;
            for c in containers.iter().filter(|c| c.state == "running") {
                let _ = state.docker.restart_container(&c.id).await;
                restarted += 1;
            }
            serde_json::json!({ "restarted": restarted, "message": format!("Restarted {restarted} container(s)") })
        }
        "diagnose" => {
            let id = str_param(p, "app_id")?;
            let app = state
                .db
                .get_app(&id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("app {id}")))?;
            let deploys = state.db.list_deploys(&id, 5).await?;
            let health_checks = state.db.get_health_checks(&id).await?;
            let logs = state.log_store.search(&id, None, None, 50).await;
            let metrics = state.server_metrics.read().await;

            let latest_status = deploys
                .first()
                .map(|d| d.status.as_str())
                .unwrap_or("unknown");
            let health_status = if health_checks.is_empty() {
                "no checks configured"
            } else {
                "configured"
            };

            serde_json::json!({
                "app": { "name": app.name, "deploy_mode": app.deploy_mode, "server_id": app.server_id },
                "latest_deploy_status": latest_status,
                "recent_deploys": deploys.len(),
                "health": health_status,
                "log_lines": logs.len(),
                "recent_logs": logs.into_iter().take(20).collect::<Vec<_>>(),
                "server_cpu_percent": metrics.cpu_percent,
                "server_memory_used_gb": metrics.memory_used_bytes as f64 / 1e9,
            })
        }
        "suggest_fix" => {
            let id = str_param(p, "app_id")?;
            let issue = p.get("issue").and_then(|v| v.as_str()).unwrap_or("unknown");
            let deploys = state.db.list_deploys(&id, 3).await?;
            let latest = deploys.first();

            let mut suggestions = Vec::new();
            if let Some(d) = latest {
                if d.status == "failed" {
                    suggestions.push("Recent deploy failed. Try: rollback to the previous successful deploy, or check build logs for errors.");
                }
            }
            if issue.contains("crash") || issue.contains("restart") || issue.contains("OOM") {
                suggestions.push("Container may be running out of memory. Check resource limits and increase if needed.");
            }
            if issue.contains("slow") || issue.contains("timeout") || issue.contains("latency") {
                suggestions.push("Check CPU usage and recent deploy changes. Consider scaling up or investigating the latest code changes.");
            }
            if issue.contains("502") || issue.contains("unhealthy") || issue.contains("health") {
                suggestions.push("Health check is failing. Verify the health endpoint is responding and the container is running.");
            }
            if suggestions.is_empty() {
                suggestions.push("Check recent logs for errors. Restart the app if the issue persists. Review recent deploys for changes.");
            }

            serde_json::json!({ "suggestions": suggestions, "app_id": id, "issue": issue })
        }
        "cancel_deploy" => {
            let deploy_id = str_param(p, "deploy_id")?;
            let deploy = state
                .db
                .get_deploy(&deploy_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;
            if !["pending", "building", "deploying"].contains(&deploy.status.as_str()) {
                return Err(ApiError::BadRequest(format!(
                    "Cannot cancel deploy with status '{}'",
                    deploy.status
                )));
            }
            state
                .db
                .update_deploy_status(&deploy_id, "cancelled", Some("Cancelled via MCP"))
                .await?;
            serde_json::json!({ "message": format!("Deploy {deploy_id} cancelled"), "status": "cancelled" })
        }
        "rollback_deploy" => {
            let app_id = str_param(p, "app_id")?;
            let deploy_id = str_param(p, "deploy_id")?;
            let target = state
                .db
                .get_deploy(&deploy_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;
            let image_ref = target
                .image_ref
                .ok_or_else(|| ApiError::BadRequest("Deploy has no image to rollback to".into()))?;
            let envs = state.db.list_environments(&app_id).await?;
            let env = envs
                .first()
                .ok_or_else(|| ApiError::BadRequest("No environments".into()))?;
            let rollback = state
                .db
                .create_deploy(&crate::db::models::NewDeploy {
                    app_id,
                    environment_id: env.id.clone(),
                    git_sha: target.git_sha,
                    server_id: None,
                    tag: None,
                    no_cache: false,
                })
                .await?;
            serde_json::json!({ "deploy_id": rollback.id, "message": "Rollback triggered", "image": image_ref })
        }
        "approve_deploy" => {
            let deploy_id = str_param(p, "deploy_id")?;
            let deploy = state
                .db
                .get_deploy(&deploy_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("deploy {deploy_id}")))?;
            if deploy.status != "pending_approval" {
                return Err(ApiError::BadRequest(format!(
                    "Deploy is '{}', not pending_approval",
                    deploy.status
                )));
            }
            state
                .db
                .create_deploy_approval(&deploy_id, "approved", &user.id, Some("Approved via MCP"))
                .await?;
            state
                .db
                .update_deploy_status(&deploy_id, "pending", Some("Approved via MCP"))
                .await?;
            serde_json::json!({ "message": "Deploy approved", "deploy_id": deploy_id })
        }
        "bulk_restart" => {
            let apps = state.db.list_apps().await?;
            let project_filter = p.get("project_id").and_then(|v| v.as_str());
            let tag_filter = p.get("tag").and_then(|v| v.as_str());
            let filtered: Vec<_> = apps
                .iter()
                .filter(|a| {
                    if let Some(pid) = project_filter {
                        a.project_id.as_deref() == Some(pid)
                    } else if let Some(tag) = tag_filter {
                        a.tags.as_deref().is_some_and(|t| t.contains(tag))
                    } else {
                        false
                    }
                })
                .collect();
            let mut restarted = 0;
            for app in &filtered {
                let label = format!("icefall.app={}", app.id);
                if let Ok(containers) = state.docker.list_containers(Some(&label)).await {
                    for c in containers.iter().filter(|c| c.state == "running") {
                        let _ = state.docker.restart_container(&c.id).await;
                        restarted += 1;
                    }
                }
            }
            serde_json::json!({ "apps_matched": filtered.len(), "containers_restarted": restarted })
        }
        "bulk_deploy" => {
            let apps = state.db.list_apps().await?;
            let project_filter = p.get("project_id").and_then(|v| v.as_str());
            let tag_filter = p.get("tag").and_then(|v| v.as_str());
            let filtered: Vec<_> = apps
                .iter()
                .filter(|a| {
                    if let Some(pid) = project_filter {
                        a.project_id.as_deref() == Some(pid)
                    } else if let Some(tag) = tag_filter {
                        a.tags.as_deref().is_some_and(|t| t.contains(tag))
                    } else {
                        false
                    }
                })
                .collect();
            let mut deploy_ids = Vec::new();
            for app in &filtered {
                let envs = state
                    .db
                    .list_environments(&app.id)
                    .await
                    .unwrap_or_default();
                if let Some(env) = envs.first() {
                    if let Ok(deploy) = state
                        .db
                        .create_deploy(&crate::db::models::NewDeploy {
                            app_id: app.id.clone(),
                            environment_id: env.id.clone(),
                            git_sha: None,
                            server_id: None,
                            tag: None,
                            no_cache: false,
                        })
                        .await
                    {
                        deploy_ids.push(deploy.id);
                    }
                }
            }
            serde_json::json!({ "apps_matched": filtered.len(), "deploys_triggered": deploy_ids.len(), "deploy_ids": deploy_ids })
        }
        "create_app" => {
            let name = str_param(p, "name")?;
            let git_repo = p.get("git_repo").and_then(|v| v.as_str()).map(String::from);
            let branch = p.get("branch").and_then(|v| v.as_str()).unwrap_or("main");
            let framework = p
                .get("framework")
                .and_then(|v| v.as_str())
                .map(String::from);
            let app = state
                .db
                .create_app(&crate::db::models::NewApp {
                    name: name.clone(),
                    // The app belongs to the caller's active team.
                    team_id: ctx.team_id.clone(),
                    git_repo,
                    git_branch: branch.to_string(),
                    framework,
                    image_ref: None,
                    compose_content: None,
                    deploy_mode: None,
                    server_id: None,
                })
                .await?;
            let _ = state
                .db
                .create_environment(&crate::db::models::NewEnvironment {
                    app_id: app.id.clone(),
                    name: "production".into(),
                    env_type: "production".into(),
                    branch: None,
                })
                .await;
            serde_json::json!({ "app_id": app.id, "name": app.name, "message": format!("App '{name}' created") })
        }
        "list_servers" => {
            let servers = state.db.list_servers().await?;
            serde_json::json!({ "servers": servers })
        }
        "server_forecast" => {
            let sid = str_param(p, "server_id")?;
            let data = state.db.get_server_metrics_for_forecast(&sid, 30).await?;
            if data.len() < 7 {
                serde_json::json!({ "message": "Not enough data (need 7+ days)", "days_available": data.len() })
            } else {
                let last_disk = data.last().map(|(d, _, _)| *d).unwrap_or(0.0);
                let last_mem = data.last().map(|(_, m, _)| *m).unwrap_or(0.0);
                serde_json::json!({ "disk_usage_ratio": last_disk, "memory_usage_ratio": last_mem, "data_points": data.len() })
            }
        }
        "export_bundle" => {
            let id = str_param(p, "app_id")?;
            let app = state
                .db
                .get_app(&id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("app {id}")))?;
            serde_json::json!({
                "icefall_bundle": "1.0",
                "app": { "name": app.name, "repo_url": app.git_repo, "branch": app.git_branch, "framework": app.framework, "deploy_mode": app.deploy_mode },
            })
        }
        "search" => {
            let q = str_param(p, "query")?;
            let results = state.db.search(&q).await?;
            results
        }
        "get_analytics" => {
            let days = p.get("days").and_then(|v| v.as_i64()).unwrap_or(30);
            let from = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
            let to = crate::db::models::now_iso8601();
            state.db.get_deploy_analytics(&from, &to).await?
        }
        _ => {
            return Err(ApiError::BadRequest(format!("Unknown tool: {}", body.tool)));
        }
    };

    Ok(Json(serde_json::json!({ "result": result })))
}

fn tool_def(name: &str, description: &str, params: &[serde_json::Value]) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": params.iter().map(|p| {
                let n = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                (n.to_string(), serde_json::json!({
                    "type": p.get("type").and_then(|v| v.as_str()).unwrap_or("string"),
                    "description": p.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                }))
            }).collect::<serde_json::Map<String, serde_json::Value>>(),
        }
    })
}

fn param(name: &str, typ: &str, description: &str) -> serde_json::Value {
    serde_json::json!({ "name": name, "type": typ, "description": description })
}

fn str_param(params: &serde_json::Value, key: &str) -> Result<String, ApiError> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string)
        .ok_or_else(|| ApiError::BadRequest(format!("Missing required parameter: {key}")))
}

// IF-196: MCP Resources protocol

async fn list_resources() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "resources": [
            { "uri": "icefall://apps", "name": "Applications", "description": "All deployed applications" },
            { "uri": "icefall://databases", "name": "Databases", "description": "All managed databases" },
            { "uri": "icefall://servers", "name": "Servers", "description": "All connected servers" },
            { "uri": "icefall://deploys/recent", "name": "Recent Deploys", "description": "Last 10 deploys across all apps" },
            { "uri": "icefall://incidents", "name": "Incidents", "description": "Active and recent incidents" },
            { "uri": "icefall://settings", "name": "Settings", "description": "Platform settings (non-sensitive)" },
        ]
    }))
}

#[derive(Deserialize)]
struct ResourceReadRequest {
    uri: String,
}

async fn read_resource(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ResourceReadRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Authentication required".into()))?;

    let data = match body.uri.as_str() {
        "icefall://apps" => {
            let apps = state.db.list_apps().await?;
            serde_json::json!(apps)
        }
        "icefall://databases" => {
            let dbs = state.db.list_managed_dbs().await?;
            serde_json::json!(dbs)
        }
        "icefall://servers" => {
            let servers = state.db.list_servers().await?;
            serde_json::json!(servers)
        }
        "icefall://deploys/recent" => {
            let apps = state.db.list_apps().await?;
            let app_ids: Vec<String> = apps.iter().map(|a| a.id.clone()).collect();
            let deploys = state.db.get_latest_deploys_for_apps(&app_ids).await?;
            serde_json::json!(deploys)
        }
        "icefall://incidents" => {
            let incidents = state.db.list_incidents(20).await?;
            serde_json::json!(incidents)
        }
        "icefall://settings" => {
            serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
            })
        }
        uri if uri.starts_with("icefall://apps/") => {
            let app_id = uri.strip_prefix("icefall://apps/").unwrap_or("");
            if let Some(suffix) = app_id.strip_suffix("/logs") {
                let logs = state.log_store.search(suffix, None, None, 100).await;
                serde_json::json!({ "logs": logs })
            } else {
                let app = state
                    .db
                    .get_app(app_id)
                    .await?
                    .ok_or_else(|| ApiError::NotFound(format!("app {app_id}")))?;
                serde_json::json!(app)
            }
        }
        uri if uri.starts_with("icefall://servers/") => {
            let parts: Vec<&str> = uri
                .strip_prefix("icefall://servers/")
                .unwrap_or("")
                .split('/')
                .collect();
            if let Some(&server_id) = parts.first() {
                if parts.get(1) == Some(&"metrics") {
                    let metrics = state.server_metrics.read().await;
                    serde_json::json!({
                        "cpu_percent": metrics.cpu_percent,
                        "memory_used_bytes": metrics.memory_used_bytes,
                        "memory_total_bytes": metrics.memory_total_bytes,
                        "disk_used_bytes": metrics.disk_used_bytes,
                        "disk_total_bytes": metrics.disk_total_bytes,
                    })
                } else {
                    let server = state
                        .db
                        .get_server(server_id)
                        .await?
                        .ok_or_else(|| ApiError::NotFound(format!("server {server_id}")))?;
                    serde_json::json!(server)
                }
            } else {
                return Err(ApiError::BadRequest("Invalid resource URI".into()));
            }
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unknown resource: {}",
                body.uri
            )));
        }
    };

    Ok(Json(
        serde_json::json!({ "contents": [{ "uri": body.uri, "data": data }] }),
    ))
}

// IF-196: MCP Prompts protocol

async fn list_prompts() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "prompts": [
            {
                "name": "deploy-app",
                "description": "Deploy an application with guided options",
                "arguments": [
                    { "name": "app_name", "description": "Name of the app to deploy", "required": true },
                    { "name": "branch", "description": "Branch or tag (default: configured branch)", "required": false },
                ]
            },
            {
                "name": "troubleshoot",
                "description": "Diagnose an unhealthy or failing application",
                "arguments": [
                    { "name": "app_name", "description": "Name of the app having issues", "required": true },
                    { "name": "symptom", "description": "What's wrong (e.g., 'slow', '502 errors', 'crashes')", "required": false },
                ]
            },
            {
                "name": "create-app",
                "description": "Set up a new application step by step",
                "arguments": [
                    { "name": "repo_url", "description": "Git repository URL", "required": true },
                    { "name": "name", "description": "App name (auto-generated if omitted)", "required": false },
                ]
            },
            {
                "name": "security-audit",
                "description": "Check for common security issues across all apps",
                "arguments": []
            },
            {
                "name": "setup-monitoring",
                "description": "Configure health checks and alerts for an app",
                "arguments": [
                    { "name": "app_name", "description": "Name of the app", "required": true },
                ]
            },
        ]
    }))
}

#[derive(Deserialize)]
struct PromptGetRequest {
    name: String,
    arguments: Option<serde_json::Value>,
}

async fn get_prompt(
    Json(body): Json<PromptGetRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let args = body.arguments.unwrap_or(serde_json::json!({}));

    let messages = match body.name.as_str() {
        "deploy-app" => {
            let app = args
                .get("app_name")
                .and_then(|v| v.as_str())
                .unwrap_or("my-app");
            let branch = args.get("branch").and_then(|v| v.as_str());
            let mut msg = format!("Deploy the app '{app}'.");
            if let Some(b) = branch {
                msg.push_str(&format!(" Use branch/tag: {b}."));
            }
            msg.push_str(" Steps: 1) Use get_app to verify it exists. 2) Use deploy_app to trigger the deploy. 3) Use get_deploy_status to monitor progress. 4) If it fails, use diagnose to investigate.");
            vec![serde_json::json!({ "role": "user", "content": msg })]
        }
        "troubleshoot" => {
            let app = args
                .get("app_name")
                .and_then(|v| v.as_str())
                .unwrap_or("my-app");
            let symptom = args
                .get("symptom")
                .and_then(|v| v.as_str())
                .unwrap_or("not working");
            vec![serde_json::json!({ "role": "user", "content": format!(
                "The app '{app}' is having issues: {symptom}. Please diagnose it. Steps: 1) Use diagnose tool with app_id. 2) Review logs, health status, and recent deploys. 3) Use suggest_fix with the findings. 4) Recommend specific actions."
            ) })]
        }
        "create-app" => {
            let repo = args
                .get("repo_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://github.com/user/repo");
            let name = args.get("name").and_then(|v| v.as_str());
            let mut msg = format!("Create a new app from {repo}.");
            if let Some(n) = name {
                msg.push_str(&format!(" Name it '{n}'."));
            }
            msg.push_str(" Steps: 1) Use create_app with the repo URL. 2) Set environment variables with set_env_var. 3) Add a domain with add_domain. 4) Trigger the first deploy with deploy_app.");
            vec![serde_json::json!({ "role": "user", "content": msg })]
        }
        "security-audit" => {
            vec![serde_json::json!({ "role": "user", "content":
                "Run a security audit. Steps: 1) Use list_apps to get all apps. 2) For each app, check: resource limits set? Health checks configured? Domains using HTTPS? 3) Use get_server_status to check disk/memory usage. 4) Report findings with severity levels."
            })]
        }
        "setup-monitoring" => {
            let app = args
                .get("app_name")
                .and_then(|v| v.as_str())
                .unwrap_or("my-app");
            vec![serde_json::json!({ "role": "user", "content": format!(
                "Set up monitoring for '{app}'. Steps: 1) Use get_app to check current config. 2) Use get_health_status to see existing checks. 3) If no health check, recommend adding one. 4) Verify notification channels are configured."
            ) })]
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unknown prompt: {}",
                body.name
            )));
        }
    };

    Ok(Json(serde_json::json!({ "messages": messages })))
}
