use std::path::Path;

use tracing::{error, info};

use crate::db::Database;
use crate::docker::DockerClient;
use crate::update::UpdateError;

use super::UpdateApplier;

pub async fn post_update_check(
    data_dir: &Path,
    db: &dyn Database,
    docker: &DockerClient,
) -> Result<(), UpdateError> {
    let applier = UpdateApplier::new(data_dir);

    let Some(marker) = applier.read_pending_marker() else {
        return Ok(());
    };

    info!(
        from = marker.from_version,
        to = marker.to_version,
        "detected pending update, running post-update health checks"
    );

    let docker_ok = docker.ping().await.is_ok();
    if !docker_ok {
        error!("post-update health check failed: Docker is unreachable");
        return Err(UpdateError::Apply(
            "post-update health check failed: Docker unreachable".to_string(),
        ));
    }

    let db_ok = db.list_projects().await.is_ok();
    if !db_ok {
        error!("post-update health check failed: database query failed");
        return Err(UpdateError::Apply(
            "post-update health check failed: database query failed".to_string(),
        ));
    }

    info!(
        version = marker.to_version,
        "post-update health checks passed, update complete"
    );
    applier.clear_pending_marker()?;

    Ok(())
}
