use std::collections::HashMap;

use bollard::models::{
    ContainerCreateBody, ContainerInspectResponse, ContainerSummary, DeviceMapping, HostConfig,
    PortBinding,
};
use bollard::query_parameters::{
    CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, StopContainerOptions,
};
use serde::{Deserialize, Serialize};

use crate::docker::{DockerClient, DockerError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub name: String,
    pub image: String,
    pub env: Vec<String>,
    pub cmd: Option<Vec<String>>,
    pub ports: Vec<PortMapping>,
    pub volumes: Vec<VolumeMount>,
    pub memory_bytes: Option<i64>,
    pub cpu_shares: Option<i64>,
    pub restart_policy: Option<String>,
    pub labels: HashMap<String, String>,
    pub network: Option<String>,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub labels: HashMap<String, String>,
}

impl DockerClient {
    pub async fn create_container(&self, config: &ContainerConfig) -> Result<String, DockerError> {
        let quirks = self.quirks();

        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
        let mut exposed_ports: Vec<String> = Vec::new();

        for port in &config.ports {
            // Rootless Podman cannot publish privileged ports — fail early
            // with a clear message rather than a cryptic runtime error.
            if let Some(host_port) = port.host_port {
                if quirks.rootless && host_port < quirks.min_unprivileged_port {
                    return Err(DockerError::Unavailable(format!(
                        "rootless Podman cannot bind host port {host_port} \
                         (ports below {} require root); use a higher port",
                        quirks.min_unprivileged_port
                    )));
                }
            }

            let key = format!("{}/{}", port.container_port, port.protocol);
            exposed_ports.push(key.clone());
            port_bindings.insert(
                key,
                Some(vec![PortBinding {
                    // Docker / rootful Podman bind 0.0.0.0; rootless Podman uses
                    // loopback (Caddy is co-located and proxies to it).
                    host_ip: Some(quirks.host_bind_ip.clone()),
                    host_port: port.host_port.map(|p| p.to_string()),
                }]),
            );
        }

        let binds: Vec<String> = config
            .volumes
            .iter()
            .map(|v| {
                if v.read_only {
                    format!("{}:{}:ro", v.source, v.target)
                } else {
                    format!("{}:{}", v.source, v.target)
                }
            })
            .collect();

        let restart_policy =
            config
                .restart_policy
                .as_deref()
                .map(|policy| bollard::models::RestartPolicy {
                    name: Some(match policy {
                        "always" => bollard::models::RestartPolicyNameEnum::ALWAYS,
                        "unless-stopped" => bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED,
                        "on-failure" => bollard::models::RestartPolicyNameEnum::ON_FAILURE,
                        _ => bollard::models::RestartPolicyNameEnum::NO,
                    }),
                    maximum_retry_count: None,
                });

        // Rootless Podman realizes restart policies via systemd user units,
        // which only auto-start after a reboot if lingering is enabled.
        if quirks.rootless
            && matches!(
                config.restart_policy.as_deref(),
                Some("always" | "unless-stopped")
            )
        {
            tracing::warn!(
                container = %config.name,
                "rootless Podman: restart policy '{}' requires `loginctl enable-linger` \
                 to survive a host reboot",
                config.restart_policy.as_deref().unwrap_or_default()
            );
        }

        // Rootless Podman ignores cgroup limits without cgroups v2 delegation.
        if !quirks.supports_cgroup_limits
            && (config.memory_bytes.is_some() || config.cpu_shares.is_some())
        {
            tracing::warn!(
                container = %config.name,
                "runtime may ignore memory/CPU limits — rootless Podman needs \
                 cgroups v2 with delegation enabled for resource limits to apply"
            );
        }

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            binds: Some(binds),
            memory: config.memory_bytes,
            cpu_shares: config.cpu_shares,
            restart_policy,
            ..Default::default()
        };

        let container_config = ContainerCreateBody {
            image: Some(config.image.clone()),
            env: Some(config.env.clone()),
            cmd: config.cmd.clone(),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            labels: Some(config.labels.clone()),
            hostname: config.hostname.clone(),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: Some(config.name.clone()),
            ..Default::default()
        };

        let response = self
            .inner()
            .create_container(Some(options), container_config)
            .await?;

        if let Some(ref network) = config.network {
            self.connect_container_to_network(network, &response.id)
                .await?;
        }

        Ok(response.id)
    }

    /// Create a container with `SYS_ADMIN` capability and `/dev/fuse` device
    /// access, required for FUSE-based mounts like rclone.
    pub async fn create_s3_sidecar_container(
        &self,
        config: &ContainerConfig,
    ) -> Result<String, DockerError> {
        // Use `shared` bind propagation so the FUSE mount is visible
        // to other containers sharing the same volume.
        let binds: Vec<String> = config
            .volumes
            .iter()
            .map(|v| {
                if v.read_only {
                    format!("{}:{}:ro,shared", v.source, v.target)
                } else {
                    format!("{}:{}:shared", v.source, v.target)
                }
            })
            .collect();

        let restart_policy =
            config
                .restart_policy
                .as_deref()
                .map(|policy| bollard::models::RestartPolicy {
                    name: Some(match policy {
                        "always" => bollard::models::RestartPolicyNameEnum::ALWAYS,
                        "unless-stopped" => bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED,
                        "on-failure" => bollard::models::RestartPolicyNameEnum::ON_FAILURE,
                        _ => bollard::models::RestartPolicyNameEnum::NO,
                    }),
                    maximum_retry_count: None,
                });

        let host_config = HostConfig {
            binds: Some(binds),
            restart_policy,
            cap_add: Some(vec!["SYS_ADMIN".to_string()]),
            devices: Some(vec![DeviceMapping {
                path_on_host: Some("/dev/fuse".to_string()),
                path_in_container: Some("/dev/fuse".to_string()),
                cgroup_permissions: Some("rwm".to_string()),
            }]),
            ..Default::default()
        };

        let container_config = ContainerCreateBody {
            image: Some(config.image.clone()),
            env: Some(config.env.clone()),
            cmd: config.cmd.clone(),
            host_config: Some(host_config),
            labels: Some(config.labels.clone()),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: Some(config.name.clone()),
            ..Default::default()
        };

        let response = self
            .inner()
            .create_container(Some(options), container_config)
            .await?;

        Ok(response.id)
    }

    pub async fn start_container(&self, id: &str) -> Result<(), DockerError> {
        self.inner().start_container(id, None).await?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str, timeout: Option<i64>) -> Result<(), DockerError> {
        let options = timeout.map(|t| StopContainerOptions {
            t: Some(t as i32),
            ..Default::default()
        });
        self.inner().stop_container(id, options).await?;
        Ok(())
    }

    pub async fn restart_container(&self, id: &str) -> Result<(), DockerError> {
        self.inner()
            .restart_container(
                id,
                None::<bollard::query_parameters::RestartContainerOptions>,
            )
            .await?;
        Ok(())
    }

    pub async fn remove_container(&self, id: &str, force: bool) -> Result<(), DockerError> {
        let options = RemoveContainerOptions {
            force,
            ..Default::default()
        };
        self.inner().remove_container(id, Some(options)).await?;
        Ok(())
    }

    pub async fn list_containers(
        &self,
        label_filter: Option<&str>,
    ) -> Result<Vec<ContainerInfo>, DockerError> {
        let filters = label_filter.map(|label| {
            let mut f = HashMap::new();
            f.insert("label".to_string(), vec![label.to_string()]);
            f
        });

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers: Vec<ContainerSummary> = self.inner().list_containers(Some(options)).await?;

        let infos = containers
            .into_iter()
            .map(|c| ContainerInfo {
                id: c.id.unwrap_or_default(),
                name: c
                    .names
                    .and_then(|n| n.first().cloned())
                    .unwrap_or_default()
                    .trim_start_matches('/')
                    .to_string(),
                image: c.image.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                state: c.state.map(|s| s.to_string()).unwrap_or_default(),
                labels: c.labels.unwrap_or_default(),
            })
            .collect();

        Ok(infos)
    }

    pub async fn inspect_container(
        &self,
        id: &str,
    ) -> Result<ContainerInspectResponse, DockerError> {
        let info = self.inner().inspect_container(id, None).await?;
        Ok(info)
    }

    pub async fn exec_in_container(
        &self,
        container: &str,
        cmd: &[String],
    ) -> Result<String, DockerError> {
        use bollard::container::LogOutput;
        use bollard::exec::{CreateExecOptions, StartExecResults};
        use futures_util::StreamExt;

        let exec = self
            .inner()
            .create_exec(
                container,
                CreateExecOptions {
                    cmd: Some(cmd.to_vec()),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await?;

        let mut output = String::new();
        if let StartExecResults::Attached {
            output: mut exec_output,
            ..
        } = self.inner().start_exec(&exec.id, None).await?
        {
            while let Some(Ok(msg)) = exec_output.next().await {
                match msg {
                    LogOutput::StdOut { message } => {
                        output.push_str(&String::from_utf8_lossy(&message));
                    }
                    LogOutput::StdErr { message } => {
                        output.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        Ok(output)
    }
}
