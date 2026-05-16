use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::CleanupSchedule;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cleanup-schedule", get(get_schedule).put(update_schedule))
        .route("/cleanup-schedule/run", post(run_cleanup))
        .route("/cleanup-schedule/history", get(list_history))
}

fn default_schedule() -> serde_json::Value {
    serde_json::json!({
        "id": "singleton",
        "cron_expression": "0 2 * * 0",
        "disk_threshold_percent": 80,
        "cleanup_dangling_images": true,
        "cleanup_unused_images": false,
        "cleanup_stopped_containers": true,
        "stopped_container_age_hours": 48,
        "cleanup_unused_volumes": false,
        "cleanup_unused_networks": false,
        "enabled": false,
        "updated_at": "",
    })
}

async fn get_schedule(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let schedule = state.db.get_cleanup_schedule().await?;
    match schedule {
        Some(s) => Ok(Json(serde_json::json!({ "data": s }))),
        None => Ok(Json(serde_json::json!({ "data": default_schedule() }))),
    }
}

#[derive(Deserialize)]
struct UpdateScheduleRequest {
    cron: Option<String>,
    disk_threshold_percent: Option<i32>,
    dangling_images: Option<bool>,
    unused_images: Option<bool>,
    stopped_containers: Option<bool>,
    stopped_container_age_hours: Option<i32>,
    volumes: Option<bool>,
    networks: Option<bool>,
    enabled: Option<bool>,
}

async fn update_schedule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateScheduleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    // Load existing or use defaults
    let existing = state.db.get_cleanup_schedule().await?;
    let ex = existing.as_ref();

    let schedule = CleanupSchedule {
        id: "singleton".to_string(),
        enabled: body
            .enabled
            .unwrap_or_else(|| ex.is_some_and(|e| e.enabled)),
        cron_expression: body.cron.unwrap_or_else(|| {
            ex.map_or_else(|| "0 2 * * 0".to_string(), |e| e.cron_expression.clone())
        }),
        disk_threshold_percent: body
            .disk_threshold_percent
            .unwrap_or_else(|| ex.map_or(80, |e| e.disk_threshold_percent)),
        cleanup_dangling_images: body
            .dangling_images
            .unwrap_or_else(|| ex.is_none_or(|e| e.cleanup_dangling_images)),
        cleanup_unused_images: body
            .unused_images
            .unwrap_or_else(|| ex.is_some_and(|e| e.cleanup_unused_images)),
        cleanup_stopped_containers: body
            .stopped_containers
            .unwrap_or_else(|| ex.is_none_or(|e| e.cleanup_stopped_containers)),
        cleanup_unused_volumes: body
            .volumes
            .unwrap_or_else(|| ex.is_some_and(|e| e.cleanup_unused_volumes)),
        cleanup_unused_networks: body
            .networks
            .unwrap_or_else(|| ex.is_some_and(|e| e.cleanup_unused_networks)),
        stopped_container_age_hours: body
            .stopped_container_age_hours
            .unwrap_or_else(|| ex.map_or(48, |e| e.stopped_container_age_hours)),
        updated_at: String::new(), // Will be set by the DB layer
    };

    let saved = state.db.upsert_cleanup_schedule(&schedule).await?;
    Ok(Json(serde_json::json!({
        "data": saved,
        "message": "Schedule updated",
    })))
}

async fn run_cleanup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let schedule = state.db.get_cleanup_schedule().await?;
    let config = schedule.unwrap_or(CleanupSchedule {
        id: "singleton".to_string(),
        enabled: false,
        cron_expression: "0 2 * * 0".to_string(),
        disk_threshold_percent: 80,
        cleanup_dangling_images: true,
        cleanup_unused_images: false,
        cleanup_stopped_containers: true,
        cleanup_unused_volumes: false,
        cleanup_unused_networks: false,
        stopped_container_age_hours: 48,
        updated_at: String::new(),
    });

    // Record cleanup run start
    let run = state.db.create_cleanup_run().await?;

    let mut images_removed: u64 = 0;
    let mut containers_removed: u64 = 0;
    let mut volumes_removed: u64 = 0;
    let mut networks_removed: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    // Clean dangling images
    if config.cleanup_dangling_images {
        match state.docker.list_images(None).await {
            Ok(images) => {
                for image in &images {
                    // Dangling images have <none>:<none> as their tag or empty tags
                    let is_dangling = image.repo_tags.is_empty()
                        || image.repo_tags.iter().all(|t| t == "<none>:<none>");
                    if is_dangling {
                        let image_id = &image.id;
                        if let Err(e) = state.docker.remove_image(image_id).await {
                            errors.push(format!("Failed to remove image {image_id}: {e}"));
                        } else {
                            images_removed += 1;
                        }
                    }
                }
            }
            Err(e) => errors.push(format!("Failed to list images: {e}")),
        }
    }

    // Clean stopped containers
    if config.cleanup_stopped_containers {
        match state.docker.list_containers(None).await {
            Ok(containers) => {
                for container in &containers {
                    let is_stopped = container.state == "exited" || container.state == "dead";
                    if is_stopped {
                        if let Err(e) = state.docker.remove_container(&container.id, false).await {
                            errors.push(format!(
                                "Failed to remove container {}: {e}",
                                container.name
                            ));
                        } else {
                            containers_removed += 1;
                        }
                    }
                }
            }
            Err(e) => errors.push(format!("Failed to list containers: {e}")),
        }
    }

    // Clean unused volumes
    if config.cleanup_unused_volumes {
        match state.docker.list_volumes().await {
            Ok(volumes) => {
                for volume in &volumes {
                    if let Err(e) = state.docker.remove_volume(&volume.name).await {
                        // Volume in use will fail, which is expected
                        errors.push(format!("Failed to remove volume {}: {e}", volume.name));
                    } else {
                        volumes_removed += 1;
                    }
                }
            }
            Err(e) => errors.push(format!("Failed to list volumes: {e}")),
        }
    }

    // Clean unused networks
    if config.cleanup_unused_networks {
        match state.docker.list_networks().await {
            Ok(networks) => {
                // Skip built-in Docker networks that should never be removed
                let protected = ["bridge", "host", "none"];
                for network in &networks {
                    if protected.contains(&network.name.as_str()) {
                        continue;
                    }
                    if let Err(e) = state.docker.remove_network(&network.name).await {
                        // Network in use will fail, which is expected
                        errors.push(format!("Failed to remove network {}: {e}", network.name));
                    } else {
                        networks_removed += 1;
                    }
                }
            }
            Err(e) => errors.push(format!("Failed to list networks: {e}")),
        }
    }

    let status = if errors.is_empty() {
        "completed"
    } else if images_removed > 0
        || containers_removed > 0
        || volumes_removed > 0
        || networks_removed > 0
    {
        "completed_with_errors"
    } else {
        "failed"
    };

    let total_removed = images_removed + containers_removed + volumes_removed + networks_removed;
    let error_text = if errors.is_empty() {
        None
    } else {
        Some(errors.join("; "))
    };
    let details_json = serde_json::json!({
        "images_removed": images_removed,
        "containers_removed": containers_removed,
        "volumes_removed": volumes_removed,
        "networks_removed": networks_removed,
    });

    let _ = state
        .db
        .finish_cleanup_run(
            &run.id,
            status,
            0, // freed_bytes not tracked at this level
            total_removed as i64,
            error_text.as_deref(),
            Some(&details_json.to_string()),
        )
        .await;

    Ok(Json(serde_json::json!({
        "data": {
            "status": status,
            "images_removed": images_removed,
            "containers_removed": containers_removed,
            "volumes_removed": volumes_removed,
            "networks_removed": networks_removed,
            "errors": errors,
        }
    })))
}

async fn list_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Not authenticated".into()))?;

    let runs = state.db.list_cleanup_runs(10).await?;
    Ok(Json(serde_json::json!({ "data": runs })))
}
