use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::docker::containers::{ContainerConfig, VolumeMount};
use crate::docker::DockerClient;

use super::DeployError;

/// Configuration for an S3-backed volume mount parsed from the app's volumes JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3MountConfig {
    pub bucket: String,
    pub endpoint: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub region: Option<String>,
    pub target: String,
    pub read_only: bool,
}

/// Volume entry as stored in the database JSON. The `type` field discriminates
/// between local volumes and S3 mounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VolumeEntry {
    Local {
        source: String,
        target: String,
        #[serde(default)]
        read_only: bool,
    },
    S3 {
        bucket: String,
        #[serde(default)]
        endpoint: Option<String>,
        access_key: String,
        secret_key: String,
        #[serde(default)]
        region: Option<String>,
        target: String,
        #[serde(default)]
        read_only: bool,
    },
}

/// Parse the volumes JSON string from the app model into typed entries.
/// Falls back to treating entries without a `type` field as local volumes
/// for backward compatibility.
pub fn parse_volume_entries(raw: Option<&str>) -> Vec<VolumeEntry> {
    let Some(raw) = raw else { return Vec::new() };
    if raw.is_empty() {
        return Vec::new();
    }

    // First try to deserialize with the tagged enum format.
    if let Ok(entries) = serde_json::from_str::<Vec<VolumeEntry>>(raw) {
        return entries;
    }

    // Fallback: parse as the legacy format (array of { source, target, read_only }).
    if let Ok(legacy) = serde_json::from_str::<Vec<LegacyVolume>>(raw) {
        return legacy
            .into_iter()
            .map(|v| VolumeEntry::Local {
                source: v.source,
                target: v.target,
                read_only: v.read_only,
            })
            .collect();
    }

    Vec::new()
}

#[derive(Deserialize)]
struct LegacyVolume {
    source: String,
    target: String,
    #[serde(default)]
    read_only: bool,
}

/// Separate volume entries into local volume mounts (for the app container)
/// and S3 mount configs (which need sidecar containers).
pub fn split_volumes(entries: &[VolumeEntry]) -> (Vec<VolumeMount>, Vec<S3MountConfig>) {
    let mut local = Vec::new();
    let mut s3 = Vec::new();

    for entry in entries {
        match entry {
            VolumeEntry::Local {
                source,
                target,
                read_only,
            } => {
                local.push(VolumeMount {
                    source: source.clone(),
                    target: target.clone(),
                    read_only: *read_only,
                });
            }
            VolumeEntry::S3 {
                bucket,
                endpoint,
                access_key,
                secret_key,
                region,
                target,
                read_only,
            } => {
                s3.push(S3MountConfig {
                    bucket: bucket.clone(),
                    endpoint: endpoint.clone(),
                    access_key: access_key.clone(),
                    secret_key: secret_key.clone(),
                    region: region.clone(),
                    target: target.clone(),
                    read_only: *read_only,
                });
            }
        }
    }

    (local, s3)
}

/// Name for the shared Docker volume between the sidecar and the app container.
pub fn s3_volume_name(app_name: &str, index: usize) -> String {
    format!("icefall-{app_name}-s3-{index}")
}

/// Name for the sidecar container.
fn sidecar_container_name(app_name: &str, index: usize) -> String {
    format!("icefall-{app_name}-s3-sidecar-{index}")
}

/// Create an rclone sidecar container that FUSE-mounts an S3 bucket into a
/// shared Docker volume. Returns the container ID of the started sidecar.
///
/// The app container should mount the same named volume at `config.target`.
pub async fn create_s3_sidecar(
    docker: &DockerClient,
    app_id: &str,
    app_name: &str,
    index: usize,
    config: &S3MountConfig,
) -> Result<String, DeployError> {
    let volume_name = s3_volume_name(app_name, index);
    let container_name = sidecar_container_name(app_name, index);

    // Ensure the shared volume exists.
    if let Err(e) = docker.create_volume(&volume_name).await {
        tracing::debug!("Volume {volume_name} may already exist: {e}");
    }

    // Pull the rclone image if not present.
    if let Err(e) = docker.pull_image("rclone/rclone", "latest").await {
        tracing::warn!("Failed to pull rclone/rclone:latest, may use cached: {e}");
    }

    // Build environment variables for rclone remote configuration.
    let region = config.region.as_deref().unwrap_or("auto");
    let endpoint = config.endpoint.as_deref().unwrap_or("");

    let mut env = vec![
        "RCLONE_CONFIG_REMOTE_TYPE=s3".to_string(),
        "RCLONE_CONFIG_REMOTE_PROVIDER=Other".to_string(),
        format!("RCLONE_CONFIG_REMOTE_ACCESS_KEY_ID={}", config.access_key),
        format!(
            "RCLONE_CONFIG_REMOTE_SECRET_ACCESS_KEY={}",
            config.secret_key
        ),
        format!("RCLONE_CONFIG_REMOTE_REGION={region}"),
    ];
    if !endpoint.is_empty() {
        env.push(format!("RCLONE_CONFIG_REMOTE_ENDPOINT={endpoint}"));
    }

    // Build the rclone mount command.
    let bucket_path = format!("remote:{}/", config.bucket);
    let mut cmd = vec![
        "mount".to_string(),
        bucket_path,
        "/mnt/s3".to_string(),
        "--vfs-cache-mode".to_string(),
        "full".to_string(),
        "--allow-other".to_string(),
        "--daemon-wait".to_string(),
        "0".to_string(),
    ];
    if config.read_only {
        cmd.push("--read-only".to_string());
    }

    let mut labels = HashMap::new();
    labels.insert("icefall.app".to_string(), app_id.to_string());
    labels.insert("icefall.s3-sidecar".to_string(), "true".to_string());
    labels.insert("icefall.s3-sidecar-index".to_string(), index.to_string());

    // Create the sidecar container with SYS_ADMIN capability and /dev/fuse.
    let sidecar_config = ContainerConfig {
        name: container_name,
        image: "rclone/rclone:latest".to_string(),
        env,
        cmd: Some(cmd),
        ports: Vec::new(),
        volumes: vec![VolumeMount {
            source: volume_name.clone(),
            target: "/mnt/s3".to_string(),
            read_only: false,
        }],
        memory_bytes: None,
        cpu_shares: None,
        restart_policy: Some("unless-stopped".to_string()),
        labels,
        network: None,
    };

    // We need to create the container manually to inject privileged settings
    // because ContainerConfig doesn't expose cap_add / devices.
    let container_id = docker
        .create_s3_sidecar_container(&sidecar_config)
        .await
        .map_err(|e| DeployError::ContainerCreate(format!("S3 sidecar creation failed: {e}")))?;

    docker.start_container(&container_id).await?;

    tracing::info!(
        "Started S3 sidecar {container_id} for bucket {} at volume {volume_name}",
        config.bucket
    );

    Ok(container_id)
}

/// Stop and remove all S3 sidecar containers for the given app.
pub async fn stop_s3_sidecars(docker: &DockerClient, app_id: &str) {
    let label = format!("icefall.app={app_id}");
    let sidecars = match docker.list_containers(Some(&label)).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to list S3 sidecars for cleanup: {e}");
            return;
        }
    };

    for container in sidecars {
        let is_sidecar = container
            .labels
            .get("icefall.s3-sidecar")
            .is_some_and(|v| v == "true");

        if !is_sidecar {
            continue;
        }

        tracing::info!("Stopping S3 sidecar container {}", container.id);
        let _ = docker.stop_container(&container.id, Some(10)).await;
        let _ = docker.remove_container(&container.id, true).await;
    }
}

/// Remove the shared S3 Docker volumes for a given app.
pub async fn remove_s3_volumes(docker: &DockerClient, app_name: &str, count: usize) {
    for i in 0..count {
        let vol = s3_volume_name(app_name, i);
        if let Err(e) = docker.remove_volume(&vol).await {
            tracing::debug!("Could not remove S3 volume {vol}: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_legacy_volumes() {
        let raw = r#"[{"source":"data","target":"/app/data","read_only":false}]"#;
        let entries = parse_volume_entries(Some(raw));
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0], VolumeEntry::Local { .. }));
    }

    #[test]
    fn parse_mixed_volumes() {
        let raw = r#"[
            {"type":"local","source":"data","target":"/app/data","read_only":false},
            {"type":"s3","bucket":"my-bucket","endpoint":"https://s3.example.com","access_key":"ak","secret_key":"sk","region":"auto","target":"/app/s3","read_only":true}
        ]"#;
        let entries = parse_volume_entries(Some(raw));
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], VolumeEntry::Local { .. }));
        assert!(matches!(entries[1], VolumeEntry::S3 { .. }));
    }

    #[test]
    fn split_volumes_separates_correctly() {
        let entries = vec![
            VolumeEntry::Local {
                source: "data".to_string(),
                target: "/app/data".to_string(),
                read_only: false,
            },
            VolumeEntry::S3 {
                bucket: "my-bucket".to_string(),
                endpoint: Some("https://s3.example.com".to_string()),
                access_key: "ak".to_string(),
                secret_key: "sk".to_string(),
                region: Some("auto".to_string()),
                target: "/app/s3".to_string(),
                read_only: true,
            },
        ];
        let (local, s3) = split_volumes(&entries);
        assert_eq!(local.len(), 1);
        assert_eq!(s3.len(), 1);
        assert_eq!(s3[0].bucket, "my-bucket");
        assert!(s3[0].read_only);
    }

    #[test]
    fn parse_empty_returns_empty() {
        assert!(parse_volume_entries(None).is_empty());
        assert!(parse_volume_entries(Some("")).is_empty());
        assert!(parse_volume_entries(Some("not-json")).is_empty());
    }

    #[test]
    fn volume_name_format() {
        assert_eq!(s3_volume_name("myapp", 0), "icefall-myapp-s3-0");
        assert_eq!(s3_volume_name("myapp", 2), "icefall-myapp-s3-2");
    }
}
