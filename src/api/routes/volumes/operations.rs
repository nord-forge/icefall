use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::docker::DockerError;

use super::discovery::{
    parse_ls_output, resolve_volume, safe_path, sanitize_filename, VolumeEntry,
};

const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

#[derive(Deserialize)]
pub(super) struct BrowseQuery {
    path: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct DownloadQuery {
    path: String,
}

#[derive(Deserialize)]
pub(super) struct UploadQuery {
    path: String,
    filename: String,
}

#[derive(Deserialize)]
pub(super) struct DeleteBody {
    path: String,
}

pub(super) async fn list_volumes(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let volumes: Vec<VolumeEntry> = app
        .volumes
        .as_deref()
        .and_then(|v| serde_json::from_str(v).ok())
        .unwrap_or_default();

    Ok(Json(serde_json::json!({ "data": volumes })))
}

pub(super) async fn browse_volume(
    State(state): State<AppState>,
    Path((id, mount_index)): Path<(String, usize)>,
    Query(query): Query<BrowseQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (volume, container_name) = resolve_volume(&state, &id, mount_index).await?;
    let user_path = query.path.as_deref().unwrap_or("/");
    let full_path = safe_path(&volume.target, user_path)?;

    let cmd: Vec<String> = vec![
        "ls".into(),
        "-la".into(),
        "--time-style=long-iso".into(),
        full_path.clone(),
    ];

    let output = state
        .docker
        .exec_in_container(&container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let entries = parse_ls_output(&output);

    Ok(Json(serde_json::json!({
        "data": entries,
        "path": user_path,
        "mount_target": volume.target,
    })))
}

pub(super) async fn download_file(
    State(state): State<AppState>,
    Path((id, mount_index)): Path<(String, usize)>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, ApiError> {
    let (volume, container_name) = resolve_volume(&state, &id, mount_index).await?;
    let full_path = safe_path(&volume.target, &query.path)?;

    let check_cmd: Vec<String> = vec![
        "sh".into(),
        "-c".into(),
        format!("[ -f '{}' ] && echo FILE || echo NOTFILE", full_path),
    ];
    let check_output = state
        .docker
        .exec_in_container(&container_name, &check_cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;
    if check_output.trim() != "FILE" {
        return Err(ApiError::NotFound(format!(
            "File not found or is a directory: {}",
            query.path
        )));
    }

    let cmd: Vec<String> = vec!["base64".into(), full_path.clone()];

    let output = state
        .docker
        .exec_in_container(&container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        output.trim().replace('\n', ""),
    )
    .map_err(|_| {
        ApiError::Internal(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to decode file contents",
        )))
    })?;

    let filename = query.path.rsplit('/').next().unwrap_or("download");

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    Ok(response)
}

pub(super) async fn upload_file(
    State(state): State<AppState>,
    Path((id, mount_index)): Path<(String, usize)>,
    Query(query): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (volume, container_name) = resolve_volume(&state, &id, mount_index).await?;

    if volume.read_only {
        return Err(ApiError::BadRequest(
            "Cannot upload to a read-only volume".into(),
        ));
    }

    if body.len() > MAX_UPLOAD_BYTES {
        return Err(ApiError::BadRequest(format!(
            "File too large. Maximum upload size is {} MB.",
            MAX_UPLOAD_BYTES / (1024 * 1024)
        )));
    }

    let filename = sanitize_filename(&query.filename)?;
    let dest_dir = safe_path(&volume.target, &query.path)?;
    let dest_path = format!("{}/{}", dest_dir.trim_end_matches('/'), filename);

    if !dest_path.starts_with(volume.target.trim_end_matches('/')) {
        return Err(ApiError::BadRequest(
            "Destination path is outside the volume mount".into(),
        ));
    }

    let mkdir_cmd: Vec<String> = vec!["mkdir".into(), "-p".into(), dest_dir.clone()];
    let _ = state
        .docker
        .exec_in_container(&container_name, &mkdir_cmd)
        .await;

    let mut tar_buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_buf);
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, &filename, &body[..])
            .map_err(|e| ApiError::Internal(Box::new(e)))?;
        builder
            .finish()
            .map_err(|e| ApiError::Internal(Box::new(e)))?;
    }

    use bollard::query_parameters::UploadToContainerOptions;
    state
        .docker
        .inner()
        .upload_to_container(
            &container_name,
            Some(UploadToContainerOptions {
                path: dest_dir.clone(),
                ..Default::default()
            }),
            bollard::body_full(bytes::Bytes::from(tar_buf)),
        )
        .await
        .map_err(|e| ApiError::Internal(Box::new(DockerError::Api(e))))?;

    Ok(Json(serde_json::json!({
        "message": "uploaded",
        "path": dest_path,
        "size": body.len(),
    })))
}

pub(super) async fn volume_size(
    State(state): State<AppState>,
    Path((id, mount_index)): Path<(String, usize)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (volume, container_name) = resolve_volume(&state, &id, mount_index).await?;

    let cmd: Vec<String> = vec!["du".into(), "-sb".into(), volume.target.clone()];

    let output = state
        .docker
        .exec_in_container(&container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let bytes_used: u64 = output
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "data": {
            "bytes_used": bytes_used,
            "mount_target": volume.target,
        }
    })))
}

pub(super) async fn delete_file(
    State(state): State<AppState>,
    Path((id, mount_index)): Path<(String, usize)>,
    Json(body): Json<DeleteBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (volume, container_name) = resolve_volume(&state, &id, mount_index).await?;

    if volume.read_only {
        return Err(ApiError::BadRequest(
            "Cannot delete from a read-only volume".into(),
        ));
    }

    let full_path = safe_path(&volume.target, &body.path)?;

    if full_path.trim_end_matches('/') == volume.target.trim_end_matches('/') {
        return Err(ApiError::BadRequest(
            "Cannot delete the volume mount root".into(),
        ));
    }

    let cmd: Vec<String> = vec!["rm".into(), "-rf".into(), full_path.clone()];

    state
        .docker
        .exec_in_container(&container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    Ok(Json(serde_json::json!({
        "message": "deleted",
        "path": full_path,
    })))
}
