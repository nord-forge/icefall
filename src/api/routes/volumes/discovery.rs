use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct VolumeEntry {
    #[serde(default)]
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default = "default_volume_type", rename = "type")]
    pub volume_type: String,
}

fn default_volume_type() -> String {
    "local".to_string()
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct FileEntry {
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub is_dir: bool,
    pub permissions: String,
}

pub(super) fn safe_path(mount_target: &str, user_path: &str) -> Result<String, ApiError> {
    if user_path.is_empty() {
        return Ok(mount_target.to_string());
    }

    if user_path.contains('\0') {
        return Err(ApiError::BadRequest(
            "Path contains invalid characters".into(),
        ));
    }

    for segment in user_path.split('/') {
        if segment == ".." {
            return Err(ApiError::BadRequest("Path traversal is not allowed".into()));
        }
    }

    let mount = mount_target.trim_end_matches('/');
    let sub = user_path.trim_start_matches('/');
    let full = if sub.is_empty() {
        mount.to_string()
    } else {
        format!("{mount}/{sub}")
    };

    if !full.starts_with(mount) {
        return Err(ApiError::BadRequest(
            "Path is outside the volume mount".into(),
        ));
    }

    Ok(full)
}

pub(super) fn sanitize_filename(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err(ApiError::BadRequest("Invalid filename".into()));
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(ApiError::BadRequest(
            "Filename must not contain path separators".into(),
        ));
    }
    if name.len() > 255 {
        return Err(ApiError::BadRequest("Filename is too long".into()));
    }
    Ok(name.to_string())
}

pub(super) async fn resolve_volume(
    state: &AppState,
    app_id: &str,
    mount_index: usize,
) -> Result<(VolumeEntry, String), ApiError> {
    let app = state
        .db
        .get_app(app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{app_id}' not found")))?;

    let volumes_json = app
        .volumes
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("App has no volumes configured".into()))?;

    let volumes: Vec<VolumeEntry> = serde_json::from_str(volumes_json)
        .map_err(|_| ApiError::BadRequest("Invalid volumes configuration".into()))?;

    let volume = volumes
        .get(mount_index)
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Volume mount index {mount_index} not found (app has {} mounts)",
                volumes.len()
            ))
        })?
        .clone();

    if volume.volume_type == "s3" {
        return Err(ApiError::BadRequest(
            "S3 volumes cannot be browsed through the container".into(),
        ));
    }

    let label = format!("icefall.app={app_id}");
    let containers = state.docker.list_containers(Some(&label)).await?;
    let container = containers
        .iter()
        .find(|c| c.state == "running")
        .ok_or_else(|| {
            ApiError::BadRequest("No running container for this app. Start the app first.".into())
        })?;

    Ok((volume, container.name.clone()))
}

pub(super) fn parse_ls_output(output: &str) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("total") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            continue;
        }

        let permissions = parts[0];
        let name = parts[7..].join(" ");

        if name == "." || name == ".." {
            continue;
        }

        let is_dir = permissions.starts_with('d');
        let size: u64 = parts[4].parse().unwrap_or(0);
        let modified = format!("{} {}", parts[5], parts[6]);

        entries.push(FileEntry {
            name,
            size,
            modified,
            is_dir,
            permissions: permissions.to_string(),
        });
    }

    entries
}
