use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::{now_iso8601, NewUser};

const STEPS: &[&str] = &[
    "admin_account",
    "server_check",
    "base_domain",
    "git_provider",
    "first_app",
    "first_deploy",
];

const OPTIONAL_STEPS: &[&str] = &["base_domain", "git_provider", "first_app", "first_deploy"];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/onboarding/status", get(get_status))
        .route("/onboarding/{step}/complete", post(complete_step))
        .route("/onboarding/skip/{step}", post(skip_step))
        .route("/onboarding/complete", post(complete_onboarding))
        .route("/onboarding/admin", post(create_admin))
        .route("/onboarding/server-check", post(run_server_check))
        .route("/onboarding/domain", post(save_domain))
        .route("/onboarding/domain/verify", post(verify_domain))
        .route("/onboarding/app", post(create_first_app))
}

async fn get_status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let onboarding = get_or_create(&state).await?;
    Ok(Json(onboarding))
}

async fn complete_step(
    State(state): State<AppState>,
    Path(step): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !STEPS.contains(&step.as_str()) {
        return Err(ApiError::BadRequest(format!("Unknown step: {step}")));
    }
    mark_step_complete(&state, &step).await?;
    get_status(State(state)).await
}

async fn skip_step(
    State(state): State<AppState>,
    Path(step): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !OPTIONAL_STEPS.contains(&step.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Step '{step}' is required and cannot be skipped"
        )));
    }
    mark_step_complete(&state, &step).await?;
    get_status(State(state)).await
}

async fn complete_onboarding(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let onboarding = get_or_create(&state).await?;
    let completed: Vec<String> = serde_json::from_value(
        onboarding
            .get("completed_steps")
            .cloned()
            .unwrap_or_default(),
    )
    .unwrap_or_default();

    let required = STEPS.iter().filter(|s| !OPTIONAL_STEPS.contains(s));
    for step in required {
        if !completed.contains(&step.to_string()) {
            return Err(ApiError::BadRequest(format!(
                "Required step '{step}' is not complete"
            )));
        }
    }

    let now = now_iso8601();
    let json = serde_json::to_string(&completed).unwrap_or_else(|_| "[]".to_string());
    state
        .db
        .update_onboarding_state("completed", &json, Some(&now))
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    Ok(Json(serde_json::json!({
        "message": "Onboarding complete! Welcome to Icefall.",
        "completed_at": now,
    })))
}

#[derive(Deserialize)]
struct CreateAdminRequest {
    email: String,
    password: String,
}

async fn create_admin(
    State(state): State<AppState>,
    Json(body): Json<CreateAdminRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let users = state.db.list_users().await?;
    if !users.is_empty() {
        return Err(ApiError::BadRequest("Admin already exists".into()));
    }
    if body.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    let password_hash = hash_password(&body.password)?;
    let user = state
        .db
        .create_user(&NewUser {
            email: body.email.clone(),
            password_hash,
            role: "admin".to_string(),
        })
        .await?;

    let expires_at = (chrono::Utc::now() + chrono::Duration::days(7))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let session = state.db.create_session(&user.id, &expires_at).await?;

    mark_step_complete(&state, "admin_account").await?;

    Ok(Json(serde_json::json!({
        "data": {
            "user": { "id": user.id, "email": user.email, "role": user.role },
            "session_id": session.id,
        }
    })))
}

async fn run_server_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut checks = Vec::new();

    let docker_ok = state.docker.ping().await.is_ok();
    checks.push(serde_json::json!({
        "id": "docker", "name": "Docker Engine",
        "status": if docker_ok { "pass" } else { "fail" },
        "message": if docker_ok { "Connected" } else { "Not reachable. Run: sudo systemctl start docker" },
    }));

    let caddy_ok = state.caddy.health_check().await.is_ok();
    checks.push(serde_json::json!({
        "id": "caddy", "name": "Caddy",
        "status": if caddy_ok { "pass" } else { "warn" },
        "message": if caddy_ok { "Running" } else { "Not reachable. HTTPS won't work" },
    }));

    let metrics = state.server_metrics.read().await;
    let disk_free_gb = (metrics
        .disk_total_bytes
        .saturating_sub(metrics.disk_used_bytes)) as f64
        / 1e9;
    checks.push(serde_json::json!({
        "id": "disk", "name": "Disk Space",
        "status": if disk_free_gb > 5.0 { "pass" } else if disk_free_gb > 2.0 { "warn" } else { "fail" },
        "message": format!("{:.1} GB free", disk_free_gb),
    }));

    let mem_gb = metrics.memory_total_bytes as f64 / 1e9;
    checks.push(serde_json::json!({
        "id": "memory", "name": "Memory",
        "status": if mem_gb >= 1.0 { "pass" } else { "warn" },
        "message": format!("{:.1} GB total", mem_gb),
    }));

    let required_pass = checks
        .iter()
        .filter(|c| {
            matches!(
                c.get("id").and_then(|v| v.as_str()),
                Some("docker" | "disk")
            )
        })
        .all(|c| c.get("status").and_then(|v| v.as_str()) != Some("fail"));

    if required_pass {
        mark_step_complete(&state, "server_check").await?;
    }

    Ok(Json(
        serde_json::json!({ "checks": checks, "all_passed": required_pass }),
    ))
}

#[derive(Deserialize)]
struct SaveDomainRequest {
    base_domain: String,
}

async fn save_domain(
    State(state): State<AppState>,
    Json(body): Json<SaveDomainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let server_ip = crate::api::utils::detect_server_ip().await;
    let wildcard = format!("*.{}", body.base_domain);
    let _ = state.caddy.add_route(&wildcard, "localhost:0").await;

    mark_step_complete(&state, "base_domain").await?;

    Ok(Json(serde_json::json!({
        "base_domain": body.base_domain,
        "server_ip": server_ip,
        "dns_records": [
            { "type": "A", "name": &body.base_domain, "value": server_ip.as_deref().unwrap_or("YOUR_IP") },
            { "type": "A", "name": format!("*.{}", body.base_domain), "value": server_ip.as_deref().unwrap_or("YOUR_IP") },
        ]
    })))
}

async fn verify_domain(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let domain = state.config.base_domain.as_deref().unwrap_or("");
    if domain.is_empty() {
        return Ok(Json(
            serde_json::json!({"verified": false, "error": "No domain configured"}),
        ));
    }
    let ip = crate::api::utils::detect_server_ip().await;
    let ok = crate::api::utils::check_dns_points_to(domain, ip.as_deref()).await;
    Ok(Json(
        serde_json::json!({ "domain": domain, "verified": ok, "server_ip": ip }),
    ))
}

#[derive(Deserialize)]
struct CreateFirstAppRequest {
    name: String,
    git_repo: Option<String>,
    git_branch: Option<String>,
}

async fn create_first_app(
    State(state): State<AppState>,
    Json(body): Json<CreateFirstAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .create_app(&crate::db::models::NewApp {
            name: body.name,
            git_repo: body.git_repo,
            git_branch: body.git_branch.unwrap_or_else(|| "main".into()),
            framework: None,
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

    mark_step_complete(&state, "first_app").await?;

    let envs = state.db.list_environments(&app.id).await?;
    let deploy = if let Some(env) = envs.first() {
        Some(
            state
                .db
                .create_deploy(&crate::db::models::NewDeploy {
                    app_id: app.id.clone(),
                    environment_id: env.id.clone(),
                    git_sha: None,
                    server_id: None,
                })
                .await?,
        )
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "app": { "id": app.id, "name": app.name },
        "deploy": deploy.map(|d| serde_json::json!({"id": d.id, "status": d.status})),
    })))
}

async fn get_or_create(state: &AppState) -> Result<serde_json::Value, ApiError> {
    let row = state
        .db
        .get_onboarding()
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    match row {
        Some((step, completed, started, completed_at)) => {
            let steps: serde_json::Value =
                serde_json::from_str(&completed).unwrap_or(serde_json::json!([]));
            Ok(serde_json::json!({
                "current_step": step,
                "completed_steps": steps,
                "started_at": started,
                "completed_at": completed_at,
                "is_complete": completed_at.is_some(),
                "total_steps": STEPS.len(),
            }))
        }
        None => {
            let now = now_iso8601();
            state
                .db
                .create_onboarding(&now)
                .await
                .map_err(|e| ApiError::Internal(Box::new(e)))?;
            Ok(serde_json::json!({
                "current_step": "admin_account",
                "completed_steps": [],
                "started_at": now,
                "completed_at": null,
                "is_complete": false,
                "total_steps": STEPS.len(),
            }))
        }
    }
}

async fn mark_step_complete(state: &AppState, step: &str) -> Result<(), ApiError> {
    let onboarding = get_or_create(state).await?;
    let mut completed: Vec<String> = serde_json::from_value(
        onboarding
            .get("completed_steps")
            .cloned()
            .unwrap_or_default(),
    )
    .unwrap_or_default();

    if !completed.contains(&step.to_string()) {
        completed.push(step.to_string());
    }

    let next = STEPS
        .iter()
        .find(|s| !completed.contains(&s.to_string()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "completed".to_string());

    let json = serde_json::to_string(&completed).unwrap_or_else(|_| "[]".to_string());
    state
        .db
        .update_onboarding_state(&next, &json, None)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    Ok(())
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    crate::api::utils::hash_password(password)
        .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e))))
}
