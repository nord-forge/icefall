use crate::db::models::App;
use crate::db::models::Environment;
use crate::deploy::remote::RemoteExecutor;

use super::DeployManager;

impl DeployManager {
    pub(super) async fn stop_old_containers(
        &self,
        app: &App,
        env: &Environment,
        current_deploy_id: &str,
    ) {
        let label = format!("icefall.environment={}", env.id);
        let containers = match self.docker.list_containers(Some(&label)).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to list old containers: {e}");
                return;
            }
        };

        for container in containers {
            let is_current = container
                .labels
                .get("icefall.deploy-id")
                .is_some_and(|id| id == current_deploy_id);

            // Skip S3 sidecar containers — they are shared across deploys.
            let is_sidecar = container
                .labels
                .get("icefall.s3-sidecar")
                .is_some_and(|v| v == "true");

            if is_current || is_sidecar {
                continue;
            }

            tracing::info!(
                "Stopping old container {} for app {}",
                container.id,
                app.name
            );
            let _ = self
                .docker
                .stop_container(&container.id, Some(self.config.deploy_stop_timeout_secs))
                .await;
            let _ = self.docker.remove_container(&container.id, false).await;
        }
    }

    pub(super) async fn stop_old_containers_remote(
        &self,
        exec: &RemoteExecutor,
        app: &App,
        current_deploy_id: &str,
    ) {
        let containers = match exec
            .list_containers_by_label(&format!("icefall.app={}", app.id))
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to list remote containers: {e}");
                return;
            }
        };

        for c in containers {
            let c_id = match c["id"].as_str() {
                Some(id) => id.to_string(),
                None => continue,
            };

            let is_current = c["labels"]["icefall.deploy-id"]
                .as_str()
                .is_some_and(|id| id == current_deploy_id);

            if is_current {
                continue;
            }

            let belongs_to_app = c["labels"]["icefall.app"]
                .as_str()
                .is_some_and(|id| id == app.id);

            if !belongs_to_app {
                continue;
            }

            tracing::info!(
                "Stopping old remote container {} for app {}",
                c_id,
                app.name
            );
            let _ = exec
                .stop_container(&c_id, self.config.deploy_stop_timeout_secs)
                .await;
            let _ = exec.remove_container(&c_id).await;
        }
    }
}
