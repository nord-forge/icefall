//! Public TCP access for managed databases (IF-172). Allocates a port, recreates
//! the container with a loopback publish, and adds a Caddy L4 route fronting it.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::db::models::ManagedDatabase;
use crate::docker::containers::PortMapping;

use super::config::{build_db_container_config, db_configs, DbContainerSpec};

const RESOURCE_TYPE: &str = "database";

#[derive(Deserialize)]
pub(super) struct EnablePublicAccessRequest {
    /// Comma-separated IPs/CIDRs allowed to connect. Empty/omitted = open to the
    /// internet (the UI surfaces the exposure warning in that case).
    #[serde(default)]
    ip_whitelist: Option<String>,
}

/// Fetch a database the caller's team owns (admin role — public exposure is a
/// sensitive operation), or 404/403.
async fn require_db_admin(
    state: &AppState,
    ctx: &TeamCtx,
    id: &str,
) -> Result<ManagedDatabase, ApiError> {
    let db = state
        .db
        .get_managed_db_for_team(&ctx.team_id, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;
    ctx.verify_team_access(&db.team_id, TeamRole::Admin)?;
    Ok(db)
}

/// Parse and normalize the IP whitelist into a clean list of ranges. Blank
/// entries are dropped; the result is `None` when nothing usable remains.
fn parse_ip_whitelist(raw: Option<&str>) -> Option<String> {
    let cleaned: Vec<String> = raw?
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.join(","))
    }
}

/// Build the external connection string a user pastes into their DB tool. Reuses
/// the engine's template but swaps host/port for the public `host:port`.
fn public_connection_string(
    db: &ManagedDatabase,
    host: &str,
    public_port: u16,
    type_config: &super::config::DbTypeConfig,
) -> serde_json::Value {
    let creds: serde_json::Value = serde_json::from_str(&db.credentials).unwrap_or_default();
    let user = creds["user"].as_str().unwrap_or("icefall");
    let password = creds["password"].as_str().unwrap_or_default();

    // The template hardcodes the internal port; rebuild against host:public_port.
    let endpoint = format!("{host}:{public_port}");
    let url = (type_config.connection_string)(&endpoint, "", user, password);
    // The template embeds the internal port (e.g. :5432) after the host. Replace
    // that trailing ":<internal>" with the public port so the string is dialable.
    let url = rewrite_endpoint(&url, &endpoint, type_config.port);

    serde_json::json!({
        "host": host,
        "port": public_port,
        "user": user,
        "password": password,
        "url": url,
    })
}

/// Read back the loopback host port Docker assigned to the engine's published
/// port after a recreate.
async fn assigned_loopback_port(
    state: &AppState,
    container_name: &str,
    container_port: u16,
) -> Result<u16, ApiError> {
    let info = state
        .docker
        .inspect_container(container_name)
        .await
        .map_err(ApiError::internal)?;
    let key = format!("{container_port}/tcp");
    info.network_settings
        .and_then(|ns| ns.ports)
        .and_then(|ports| {
            ports
                .get(&key)
                .and_then(|b| b.as_ref())
                .and_then(|bindings| bindings.first())
                .and_then(|binding| binding.host_port.as_ref())
                .and_then(|p| p.parse::<u16>().ok())
        })
        .ok_or_else(|| {
            ApiError::internal(std::io::Error::other(format!(
                "could not determine published host port for {container_name}"
            )))
        })
}

/// Recreate the database container, optionally publishing its engine port to a
/// loopback host port. Reuses the data volume so no data is lost.
async fn recreate_db_container(
    state: &AppState,
    db: &ManagedDatabase,
    type_config: &super::config::DbTypeConfig,
    with_public: bool,
) -> Result<String, ApiError> {
    let creds: serde_json::Value = serde_json::from_str(&db.credentials).unwrap_or_default();
    let user = creds["user"].as_str().unwrap_or("icefall").to_string();
    let password = creds["password"].as_str().unwrap_or_default().to_string();

    let container_name = format!("icefall-db-{}", db.name.trim().to_lowercase());

    let extra_ports = if with_public {
        vec![PortMapping {
            container_port: type_config.port,
            host_port: None, // Docker assigns; we inspect it back afterwards.
            protocol: "tcp".to_string(),
            // Loopback-only: the DB is reachable solely through Caddy's L4 proxy,
            // which enforces the IP whitelist. Never bind 0.0.0.0 here.
            host_ip: Some("127.0.0.1".to_string()),
        }]
    } else {
        Vec::new()
    };

    // Preserve the original memory limit if we can read it; fall back to the
    // engine default. Recreate must not silently shrink the container.
    let memory_bytes = state
        .docker
        .inspect_container(&container_name)
        .await
        .ok()
        .and_then(|i| i.host_config)
        .and_then(|hc| hc.memory)
        .filter(|m| *m > 0)
        .unwrap_or(type_config.default_memory_mb * 1024 * 1024);

    let config = build_db_container_config(
        &DbContainerSpec {
            name: &db.name,
            db_type: &db.db_type,
            user: &user,
            password: &password,
            memory_bytes,
            app_id: db.app_id.as_deref(),
            extra_ports,
        },
        type_config,
    );

    // Stop + remove the old container, then create + start the new one. The
    // named volume persists independently, so the data survives.
    let _ = state.docker.stop_container(&container_name, Some(10)).await;
    state
        .docker
        .remove_container(&container_name, true)
        .await
        .map_err(ApiError::internal)?;
    let container_id = state
        .docker
        .create_container(&config)
        .await
        .map_err(ApiError::internal)?;
    state
        .docker
        .start_container(&container_id)
        .await
        .map_err(ApiError::internal)?;

    // Keep the stored container_id in sync — restart/stop look it up by name, but
    // other call sites use the id.
    let _ = state
        .db
        .update_managed_db_credentials(&db.id, &db.credentials, &container_id)
        .await;

    Ok(container_name)
}

pub(super) async fn get_public_access(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Read-only status — viewer is enough.
    let db = state
        .db
        .get_managed_db_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;
    ctx.verify_team_access(&db.team_id, TeamRole::Viewer)?;

    let Some(public) = state.db.get_public_port(&db.id).await? else {
        return Ok(Json(serde_json::json!({
            "data": { "enabled": false }
        })));
    };

    let host = state.config.base_domain.clone().unwrap_or_default();
    let configs = db_configs();
    let connection = configs
        .get(db.db_type.as_str())
        .map(|tc| public_connection_string(&db, &host, public.port as u16, tc));

    Ok(Json(serde_json::json!({
        "data": {
            "enabled": true,
            "port": public.port,
            "host": host,
            "ip_whitelist": public.ip_whitelist,
            "connection": connection,
        }
    })))
}

pub(super) async fn enable_public_access(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    Json(body): Json<EnablePublicAccessRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = require_db_admin(&state, &ctx, &id).await?;

    let configs = db_configs();
    let type_config = configs.get(db.db_type.as_str()).ok_or_else(|| {
        ApiError::BadRequest(format!("unsupported database type '{}'", db.db_type))
    })?;

    // Reject up front if this Caddy build can't do L4, to avoid allocating and
    // recreating only to fail at the proxy step.
    if !state.caddy.has_layer4_module().await {
        return Err(ApiError::BadRequest(
            "This Caddy build does not include the layer4 (caddy-l4) module, \
             which is required for public TCP access. Rebuild Caddy with caddy-l4."
                .into(),
        ));
    }

    // Idempotent: re-enabling returns the existing allocation rather than
    // double-allocating or recreating.
    if let Some(existing) = state.db.get_public_port(&db.id).await? {
        let host = state.config.base_domain.clone().unwrap_or_default();
        return Ok(Json(serde_json::json!({
            "data": {
                "enabled": true,
                "port": existing.port,
                "host": host,
                "ip_whitelist": existing.ip_whitelist,
                "connection": public_connection_string(&db, &host, existing.port as u16, type_config),
            },
            "message": "Public access already enabled"
        })));
    }

    let ip_whitelist = parse_ip_whitelist(body.ip_whitelist.as_deref());

    // Allocate a public port within the configured range.
    let settings = state.db.get_proxy_settings().await?;
    let allocated = state
        .db
        .allocate_free_public_port(
            RESOURCE_TYPE,
            &db.id,
            settings.public_port_range_start,
            settings.public_port_range_end,
            ip_whitelist.as_deref(),
        )
        .await
        .map_err(|e| match e {
            crate::db::DbError::InvalidInput(msg) => ApiError::BadRequest(msg),
            other => ApiError::internal(other),
        })?;
    let public_port = allocated.port as u16;

    // Recreate the container with a loopback publish, then learn the host port.
    let container_name = match recreate_db_container(&state, &db, type_config, true).await {
        Ok(name) => name,
        Err(e) => {
            // Roll back the allocation so a failed enable doesn't burn a port.
            let _ = state.db.release_public_port(&db.id).await;
            return Err(e);
        }
    };

    let loopback_port =
        match assigned_loopback_port(&state, &container_name, type_config.port).await {
            Ok(p) => p,
            Err(e) => {
                let _ = state.db.release_public_port(&db.id).await;
                return Err(e);
            }
        };

    // Wire the L4 route. On failure, unwind everything so we don't leave a port
    // allocated with no working proxy. `ip_whitelist` is already normalized.
    let allowed: Vec<String> = ip_whitelist
        .as_deref()
        .map(|s| s.split(',').map(|x| x.to_string()).collect())
        .unwrap_or_default();
    if let Err(e) = state
        .caddy
        .set_tcp_proxy(public_port, loopback_port, &allowed)
        .await
    {
        let _ = state.db.release_public_port(&db.id).await;
        let _ = recreate_db_container(&state, &db, type_config, false).await;
        return Err(ApiError::internal(e));
    }

    let host = state.config.base_domain.clone().unwrap_or_default();
    Ok(Json(serde_json::json!({
        "data": {
            "enabled": true,
            "port": public_port,
            "host": host,
            "ip_whitelist": ip_whitelist,
            "connection": public_connection_string(&db, &host, public_port, type_config),
        },
        "message": "Public access enabled"
    })))
}

pub(super) async fn disable_public_access(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = require_db_admin(&state, &ctx, &id).await?;

    let Some(public) = state.db.get_public_port(&db.id).await? else {
        // Already disabled — idempotent success.
        return Ok(Json(serde_json::json!({
            "data": { "enabled": false },
            "message": "Public access already disabled"
        })));
    };

    // Remove the proxy route first (idempotent), then release the port, then
    // recreate the container without the loopback publish.
    state
        .caddy
        .remove_tcp_proxy(public.port as u16)
        .await
        .map_err(ApiError::internal)?;
    state.db.release_public_port(&db.id).await?;

    let configs = db_configs();
    if let Some(type_config) = configs.get(db.db_type.as_str()) {
        // Best-effort: the proxy and allocation are already gone, so the database
        // is no longer publicly reachable even if the recreate hiccups.
        let _ = recreate_db_container(&state, &db, type_config, false).await;
    }

    Ok(Json(serde_json::json!({
        "data": { "enabled": false },
        "message": "Public access disabled"
    })))
}

/// Rewrite an engine connection-string template so host:port points at the public
/// endpoint, not the internal port. Split out for unit-testing without a DB row.
fn rewrite_endpoint(template_url: &str, endpoint: &str, internal_port: u16) -> String {
    template_url.replacen(&format!("{endpoint}:{internal_port}"), endpoint, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_internal_port_to_public_endpoint() {
        // Postgres-style: host:port followed by the internal port from the template.
        let url = "postgresql://u:p@db.example.com:10000:5432/u";
        assert_eq!(
            rewrite_endpoint(url, "db.example.com:10000", 5432),
            "postgresql://u:p@db.example.com:10000/u"
        );

        // Redis-style: no path segment after the port.
        let url = "redis://:p@db.example.com:10042:6379";
        assert_eq!(
            rewrite_endpoint(url, "db.example.com:10042", 6379),
            "redis://:p@db.example.com:10042"
        );
    }

    #[test]
    fn whitelist_trims_and_drops_blanks() {
        assert_eq!(
            parse_ip_whitelist(Some(" 1.2.3.4 , , 10.0.0.0/8 ")),
            Some("1.2.3.4,10.0.0.0/8".to_string())
        );
        assert_eq!(parse_ip_whitelist(Some("   ")), None);
        assert_eq!(parse_ip_whitelist(Some(",,")), None);
        assert_eq!(parse_ip_whitelist(None), None);
    }
}
