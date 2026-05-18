use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::utils::{detect_server_ip, resolve_domain};
use crate::api::AppState;
use crate::db::models::NewDomain;

#[derive(Deserialize)]
struct AddDomainRequest {
    domain: String,
    path: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/apps/{id}/domains", get(list_domains).post(add_domain))
        .route("/apps/{id}/domains/{domain_id}", delete(remove_domain))
        .route("/apps/{id}/domains/{domain_id}/verify", post(verify_domain))
        .route("/server/ip", get(get_server_ip))
}

async fn list_domains(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H6: read-only — the app must belong to the caller's team (viewer).
    state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let domains = state.db.list_domains(&id).await?;
    Ok(Json(serde_json::json!({ "data": domains })))
}

async fn add_domain(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    Json(body): Json<AddDomainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H6: the app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    let domain_name = body.domain.trim().to_lowercase();
    if domain_name.is_empty() {
        return Err(ApiError::BadRequest("domain name is required".into()));
    }

    let path = body
        .path
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());
    if let Some(ref p) = path {
        if !p.starts_with('/') {
            return Err(ApiError::BadRequest("path must start with /".into()));
        }
    }

    let domain = state
        .db
        .add_domain(&NewDomain {
            app_id: id,
            domain: domain_name.clone(),
            path,
        })
        .await?;

    Ok(Json(serde_json::json!({
        "data": domain,
        "dns_instructions": {
            "type": "A",
            "name": domain_name,
            "value": detect_server_ip().await.unwrap_or_else(|| "YOUR_SERVER_IP".to_string()),
            "note": "Point this domain to your server's IP address. SSL will be provisioned automatically by Caddy."
        }
    })))
}

async fn remove_domain(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path((app_id, domain_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H6: parent app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    let domains = state.db.list_domains(&app_id).await?;
    let domain = domains
        .iter()
        .find(|d| d.id == domain_id)
        .ok_or_else(|| ApiError::NotFound(format!("domain {domain_id}")))?;

    let _ = state.caddy.remove_route(&domain.domain).await;
    state.db.delete_domain(&domain_id).await?;

    Ok(Json(serde_json::json!({ "message": "deleted" })))
}

async fn verify_domain(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path((app_id, domain_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H6: parent app must belong to the caller's team, member role to mutate.
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    ctx.verify_team_access(&app.team_id, TeamRole::Member)?;

    let domains = state.db.list_domains(&app_id).await?;
    let domain = domains
        .iter()
        .find(|d| d.id == domain_id)
        .ok_or_else(|| ApiError::NotFound(format!("domain {domain_id}")))?;

    let server_ip = detect_server_ip().await;

    match resolve_domain(&domain.domain).await {
        Ok(resolved_ips) => {
            let verified = server_ip
                .as_ref()
                .is_some_and(|ip| resolved_ips.iter().any(|r| &r.to_string() == ip));

            if verified {
                state
                    .db
                    .update_domain_status(&domain_id, true, "provisioning")
                    .await?;

                if let Some(app) = state.db.get_app(&app_id).await? {
                    let containers = state
                        .docker
                        .list_containers(Some(&format!("icefall.app={}", app.id)))
                        .await
                        .unwrap_or_default();

                    if let Some(container) = containers.first() {
                        let upstream = state
                            .docker
                            .inspect_container(&container.id)
                            .await
                            .ok()
                            .and_then(|i| i.network_settings)
                            .and_then(|ns| ns.ports)
                            .and_then(|ports| {
                                ports.values().find_map(|bindings| {
                                    bindings
                                        .as_ref()?
                                        .first()?
                                        .host_port
                                        .as_ref()
                                        .map(|p| format!("localhost:{p}"))
                                })
                            });

                        if let Some(upstream) = upstream {
                            let _ = state
                                .caddy
                                .add_route_with_path(
                                    &domain.domain,
                                    domain.path.as_deref(),
                                    &upstream,
                                )
                                .await;
                            state
                                .db
                                .update_domain_status(&domain_id, true, "active")
                                .await?;
                        }
                    }
                }

                Ok(Json(serde_json::json!({
                    "verified": true,
                    "resolved_ips": resolved_ips.iter().map(std::string::ToString::to_string).collect::<Vec<_>>(),
                })))
            } else {
                Ok(Json(serde_json::json!({
                    "verified": false,
                    "resolved_ips": resolved_ips.iter().map(std::string::ToString::to_string).collect::<Vec<_>>(),
                    "expected_ip": server_ip,
                    "error": "DNS is not pointing to this server. Update your A record."
                })))
            }
        }
        Err(e) => Ok(Json(serde_json::json!({
            "verified": false,
            "error": format!("DNS lookup failed: {e}. Make sure the domain has an A record.")
        }))),
    }
}

async fn get_server_ip() -> Result<Json<serde_json::Value>, ApiError> {
    let ip = detect_server_ip().await;
    Ok(Json(serde_json::json!({ "ip": ip })))
}

#[cfg(test)]
mod tests {
    use crate::api::utils::resolve_domain;

    #[tokio::test]
    #[ignore = "requires network access — run with --include-ignored"]
    async fn resolves_known_domain() {
        let result = resolve_domain("example.com").await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "requires network access — run with --include-ignored"]
    async fn fails_on_nonexistent_domain() {
        let result = resolve_domain("this-domain-does-not-exist-icefall-test.invalid").await;
        assert!(result.is_err());
    }
}
