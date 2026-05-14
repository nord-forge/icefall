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
}

async fn list_tools() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "tools": [
            tool_def("list_apps", "List all deployed applications with their status", &[]),
            tool_def("get_app", "Get detailed information about a specific app", &[param("app_id", "string", "The app ID")]),
            tool_def("deploy_app", "Trigger a new deploy for an app", &[param("app_id", "string", "The app ID")]),
            tool_def("get_deploy_status", "Get current deploy status and recent history", &[param("app_id", "string", "The app ID")]),
            tool_def("get_logs", "Retrieve recent logs for an app", &[param("app_id", "string", "The app ID"), param("search", "string", "Search term (optional)"), param("limit", "number", "Max lines (default 100)")]),
            tool_def("set_env_var", "Set an environment variable for an app", &[param("app_id", "string", "The app ID"), param("key", "string", "Variable name"), param("value", "string", "Variable value"), param("scope", "string", "Scope: shared, production, or preview")]),
            tool_def("get_env_vars", "List environment variables for an app", &[param("app_id", "string", "The app ID")]),
            tool_def("create_database", "Provision a managed database", &[param("name", "string", "Database name"), param("db_type", "string", "Type: postgres, mysql, redis, mongo")]),
            tool_def("list_databases", "List all managed databases", &[]),
            tool_def("get_health_status", "Get health check status for an app", &[param("app_id", "string", "The app ID")]),
            tool_def("get_server_status", "Get server resource overview (CPU, memory, disk)", &[]),
            tool_def("add_domain", "Add a custom domain to an app", &[param("app_id", "string", "The app ID"), param("domain", "string", "Domain name")]),
            tool_def("restart_app", "Restart an app's container", &[param("app_id", "string", "The app ID")]),
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
    headers: HeaderMap,
    Json(body): Json<ToolCallRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest("Authentication required. Pass API token as Bearer header.".into())
        })?;

    let can_write = user.role == "admin" || user.role == "deployer";
    let write_tools = [
        "deploy_app",
        "set_env_var",
        "create_database",
        "add_domain",
        "restart_app",
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
            let deploy = state
                .db
                .create_deploy(&crate::db::models::NewDeploy {
                    app_id: id.clone(),
                    environment_id: env.id.clone(),
                    git_sha: None,
                    server_id: None,
                    tag: None,
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
