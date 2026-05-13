use std::sync::Arc;

use tracing::{error, info};

use crate::config::IcefallConfig;
use crate::db::Database;
use crate::update::discovery::UpdateChecker;
use crate::update::download::UpdateDownloader;
use crate::update::CURRENT_VERSION;

pub(super) async fn try_pre_download(
    checker: &UpdateChecker,
    db: &Arc<dyn Database>,
    config: &Arc<IcefallConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = db.get_update_state().await?;

    if !state.auto_update_enabled {
        return Ok(());
    }

    if state.available_version.is_none() {
        return Ok(());
    }

    if state.download_state == "ready" || state.download_state == "downloading" {
        return Ok(());
    }

    let version = state.available_version.as_deref().unwrap();
    info!(version, "auto-update: starting pre-download");

    db.set_update_download_state("downloading", 0, None).await?;

    let check_state = db.get_update_state().await?;
    let result = checker
        .check_for_update(CURRENT_VERSION, &check_state.channel, "0.0.0")
        .await;

    let (manifest, _info) = match result {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            let _ = db.set_update_error("Update no longer available").await;
            return Ok(());
        }
        Err(e) => {
            let _ = db.set_update_error(&e.to_string()).await;
            return Err(e.into());
        }
    };

    if manifest.breaking {
        info!("auto-update: skipping pre-download for breaking change");
        db.set_update_download_state("none", 0, None).await?;
        return Ok(());
    }

    let target = crate::update::artifact_target();
    let artifact = match manifest.artifact_for_target(target) {
        Some(a) => a.clone(),
        None => {
            let msg = format!("No artifact for target {target}");
            let _ = db.set_update_error(&msg).await;
            return Err(msg.into());
        }
    };

    let updates_dir = config.data_dir.join("updates");
    let downloader = UpdateDownloader::new(updates_dir);

    let db_progress = db.clone();
    let last_reported_pct = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(-1));
    let download_result = downloader
        .download(&artifact, version, |downloaded, total| {
            if total > 0 {
                let pct = ((downloaded as f64 / total as f64) * 100.0) as i64;
                let last = last_reported_pct.load(std::sync::atomic::Ordering::Relaxed);
                if pct >= last + 5 || pct == 100 {
                    last_reported_pct.store(pct, std::sync::atomic::Ordering::Relaxed);
                    let db_inner = db_progress.clone();
                    tokio::spawn(async move {
                        let _ = db_inner
                            .set_update_download_state("downloading", pct, None)
                            .await;
                    });
                }
            }
        })
        .await;

    match download_result {
        Ok(path) => {
            let extract_result = downloader.extract_and_validate(&path, version).await;
            match extract_result {
                Ok(binary_path) => {
                    let path_str = binary_path.to_string_lossy().to_string();
                    info!("auto-update: pre-download complete: {path_str}");
                    db.set_update_download_state("ready", 100, Some(&path_str))
                        .await?;
                    db.set_auto_update_pre_downloaded(true).await?;
                }
                Err(e) => {
                    error!("auto-update: extraction failed: {e}");
                    let _ = db.set_update_error(&e.to_string()).await;
                }
            }
        }
        Err(e) => {
            error!("auto-update: download failed: {e}");
            let _ = db.set_update_error(&e.to_string()).await;
        }
    }

    Ok(())
}
