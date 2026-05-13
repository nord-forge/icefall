use crate::db::models::{App, Environment};
use crate::deploy::remote::RemoteExecutor;
use crate::deploy::DeployError;

use super::DeployManager;

impl DeployManager {
    pub(super) async fn resolve_env_vars(
        &self,
        env: &Environment,
    ) -> Result<Vec<String>, DeployError> {
        let vars = self.db.get_env_vars(&env.id).await?;
        let mut has_port = false;
        let mut has_host = false;
        let mut result: Vec<String> = Vec::with_capacity(vars.len() + 2);
        for v in vars {
            if v.key == "PORT" {
                has_port = true;
            }
            if v.key == "HOST" {
                has_host = true;
            }
            result.push(format!("{}={}", v.key, v.value));
        }
        if !has_port {
            result.push("PORT=3000".to_string());
        }
        if !has_host {
            result.push("HOST=0.0.0.0".to_string());
        }
        Ok(result)
    }

    pub(super) fn detected_port(&self, app: &App) -> u16 {
        app.build_config
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.get("port")?.as_u64())
            .map_or(3000, |p| p as u16)
    }

    pub(super) async fn get_host_port(
        &self,
        container_id: &str,
        container_port: u16,
    ) -> Result<u16, DeployError> {
        let info = self.docker.inspect_container(container_id).await?;
        let port_key = format!("{container_port}/tcp");

        let host_port = info
            .network_settings
            .and_then(|ns| ns.ports)
            .and_then(|ports| ports.get(&port_key).cloned())
            .and_then(|bindings| bindings)
            .and_then(|bindings| bindings.first().cloned())
            .and_then(|b| b.host_port)
            .and_then(|p| p.parse::<u16>().ok())
            .ok_or_else(|| {
                DeployError::ContainerCreate(format!(
                    "could not determine host port for container {container_id}"
                ))
            })?;

        Ok(host_port)
    }

    pub(super) async fn resolve_domains(
        &self,
        app: &App,
        env: &Environment,
    ) -> Result<Vec<(String, Option<String>)>, DeployError> {
        let mut domains = Vec::new();

        if env.env_type == "preview" {
            if let (Some(ref branch), Some(ref base_domain)) =
                (&env.branch, &self.config.base_domain)
            {
                let sanitized = crate::deploy::preview::sanitize_branch_for_subdomain(branch);
                domains.push((format!("{sanitized}--{}.{base_domain}", app.name), None));
            }
        } else {
            let custom_domains = self.db.list_domains(&app.id).await?;
            for d in custom_domains {
                domains.push((d.domain, d.path));
            }

            if let Some(ref base_domain) = self.config.base_domain {
                domains.push((format!("{}.{base_domain}", app.name), None));
            }
        }

        Ok(domains)
    }

    pub(super) async fn get_remote_host_port(
        &self,
        exec: &RemoteExecutor,
        container_id: &str,
        container_port: u16,
    ) -> Result<u16, DeployError> {
        let info = exec.inspect_container(container_id).await?;
        let port_key = format!("{container_port}/tcp");

        info["network_settings"]["ports"][&port_key]
            .as_array()
            .and_then(|bindings| bindings.first())
            .and_then(|b| b["host_port"].as_str())
            .and_then(|p| p.parse::<u16>().ok())
            .ok_or_else(|| {
                DeployError::ContainerCreate(format!(
                    "could not determine host port for remote container {container_id}"
                ))
            })
    }
}
