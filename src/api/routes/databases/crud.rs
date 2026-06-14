use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::{TeamCtx, TeamRole};
use crate::api::AppState;
use crate::db::models::{NewEnvVar, NewManagedDatabase};
use crate::docker::containers::{ContainerConfig, PortMapping, VolumeMount};

use super::config::{db_configs, generate_password};

#[derive(Deserialize)]
pub(super) struct CreateDatabaseRequest {
    name: String,
    db_type: String,
    app_id: Option<String>,
    memory_mb: Option<i64>,
    expose_port: Option<bool>,
}

pub(super) async fn list_databases(
    State(state): State<AppState>,
    ctx: TeamCtx,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Scope the listing to the caller's team.
    let dbs = state.db.list_managed_dbs_by_team(&ctx.team_id).await?;
    Ok(Json(serde_json::json!({ "data": dbs })))
}

pub(super) async fn get_database(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Read-only — the database must belong to the caller's team (viewer).
    let db = state
        .db
        .get_managed_db_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;

    Ok(Json(serde_json::json!({ "data": db })))
}

pub(super) async fn create_database(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Json(body): Json<CreateDatabaseRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Creating a database requires at least member role in the team.
    ctx.verify_team_access(&ctx.team_id, TeamRole::Member)?;

    // If linking to an app, that app must belong to the caller's team.
    if let Some(ref app_id) = body.app_id {
        state
            .db
            .get_app_for_team(&ctx.team_id, app_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;
    }

    let configs = db_configs();
    let type_config = configs.get(body.db_type.as_str()).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "unsupported database type '{}'. Supported: postgres, mysql, redis, mongo",
            body.db_type
        ))
    })?;

    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("database name is required".into()));
    }

    let password = generate_password();
    let db_user = "icefall";
    let container_name = format!("icefall-db-{}", body.name.trim().to_lowercase());
    let volume_name = format!("icefall-db-{}-data", body.name.trim().to_lowercase());
    let env = (type_config.env_vars)(db_user, &password, &body.name);

    let memory_bytes = (body.memory_mb.unwrap_or(type_config.default_memory_mb)) * 1024 * 1024;

    let mut labels = HashMap::new();
    labels.insert("icefall.managed-db".to_string(), "true".to_string());
    labels.insert("icefall.db-name".to_string(), body.name.clone());

    let mut ports = Vec::new();
    if body.expose_port.unwrap_or(false) {
        ports.push(PortMapping {
            container_port: type_config.port,
            host_port: None,
            protocol: "tcp".to_string(),
        });
    }

    let data_path = match body.db_type.as_str() {
        "postgres" | "cockroachdb" => "/var/lib/postgresql/data",
        "mysql" | "mariadb" => "/var/lib/mysql",
        "redis" | "keydb" | "valkey" => "/data",
        "dragonfly" => "/data",
        "mongo" => "/data/db",
        "clickhouse" => "/var/lib/clickhouse",
        "cassandra" => "/var/lib/cassandra",
        _ => "/data",
    };

    let cmd = match body.db_type.as_str() {
        "redis" => Some(vec![
            "redis-server".to_string(),
            "--requirepass".to_string(),
            password.clone(),
        ]),
        "keydb" => Some(vec![
            "keydb-server".to_string(),
            "--requirepass".to_string(),
            password.clone(),
            "--server-threads".to_string(),
            "2".to_string(),
        ]),
        "dragonfly" => Some(vec![
            "dragonfly".to_string(),
            "--requirepass".to_string(),
            password.clone(),
        ]),
        "valkey" => Some(vec![
            "valkey-server".to_string(),
            "--requirepass".to_string(),
            password.clone(),
        ]),
        "cockroachdb" => Some(vec![
            "start-single-node".to_string(),
            "--insecure".to_string(),
            "--store=/var/lib/postgresql/data".to_string(),
        ]),
        _ => None,
    };

    let container_config = ContainerConfig {
        name: container_name.clone(),
        image: type_config.image.to_string(),
        env,
        cmd,
        ports,
        volumes: vec![VolumeMount {
            source: volume_name.clone(),
            target: data_path.to_string(),
            read_only: false,
        }],
        memory_bytes: Some(memory_bytes),
        cpu_shares: None,
        restart_policy: Some("unless-stopped".to_string()),
        labels,
        network: body
            .app_id
            .as_ref()
            .map(|_| format!("icefall-{}", body.name)),
        hostname: Some(format!("{}.icefall.internal", body.name)),
    };

    state
        .docker
        .pull_image(
            type_config
                .image
                .split(':')
                .next()
                .unwrap_or(type_config.image),
            type_config.image.split(':').nth(1).unwrap_or("latest"),
        )
        .await
        .map_err(ApiError::internal)?;

    let container_id = state
        .docker
        .create_container(&container_config)
        .await
        .map_err(ApiError::internal)?;

    state
        .docker
        .start_container(&container_id)
        .await
        .map_err(ApiError::internal)?;

    let connection_string =
        (type_config.connection_string)(&container_name, "", db_user, &password);

    let mut credentials = serde_json::json!({
        "user": db_user,
        "password": password,
        "connection_string": connection_string,
        "host": container_name,
        "port": type_config.port,
    });

    // Provision a read-only account the db_browser connects as, so a query-validation
    // bypass can't mutate the database. Provisioning failure fails the request.
    if let Some(ro) = super::readonly::provision_readonly_user(
        &state,
        &container_name,
        &body.db_type,
        db_user,
        &password,
        &body.name,
    )
    .await?
    {
        if let Some(obj) = credentials.as_object_mut() {
            obj.insert(
                "readonly".to_string(),
                serde_json::json!({ "user": ro.user, "password": ro.password }),
            );
        }
    }

    let managed_db = state
        .db
        .create_managed_db(&NewManagedDatabase {
            name: body.name,
            db_type: body.db_type,
            app_id: body.app_id.clone(),
            team_id: ctx.team_id.clone(),
        })
        .await?;

    state
        .db
        .update_managed_db_credentials(&managed_db.id, &credentials.to_string(), &container_id)
        .await?;

    if let Some(ref app_id) = body.app_id {
        let envs = state.db.list_environments(app_id).await?;
        if let Some(env) = envs.first() {
            let _ = state
                .db
                .set_env_var(&NewEnvVar {
                    environment_id: env.id.clone(),
                    key: type_config.env_var_name.to_string(),
                    value: connection_string.clone(),
                    scope: "shared".to_string(),
                })
                .await;
        }
    }

    Ok(Json(serde_json::json!({
        "data": {
            "id": managed_db.id,
            "name": managed_db.name,
            "db_type": managed_db.db_type,
            "container_id": container_id,
            "connection_string": connection_string,
            "credentials": credentials,
        }
    })))
}

pub(super) async fn delete_database(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Destructive — database must belong to the caller's team, admin role.
    let db = state
        .db
        .get_managed_db_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;
    ctx.verify_team_access(&db.team_id, TeamRole::Admin)?;

    let container_name = format!("icefall-db-{}", db.name.to_lowercase());
    let _ = state.docker.stop_container(&container_name, Some(10)).await;
    let _ = state.docker.remove_container(&container_name, true).await;

    state.db.delete_managed_db(&id).await?;

    Ok(Json(serde_json::json!({ "message": "deleted" })))
}
