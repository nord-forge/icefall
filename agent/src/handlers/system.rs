use std::path::Path;

use serde_json::Value;
use tracing::{error, info};

use crate::context::HandlerContext;
use crate::handlers::HandlerError;

pub async fn update(_ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let version = params["version"]
        .as_str()
        .ok_or_else(|| HandlerError::InvalidParams("missing version".into()))?;

    let target = params["target"]
        .as_str()
        .unwrap_or("x86_64-unknown-linux-musl");

    let current = env!("CARGO_PKG_VERSION");
    if version == current {
        return Ok(serde_json::json!({ "status": "up_to_date", "version": current }));
    }

    info!(from = current, to = version, target, "agent update requested");

    let download_url = params["download_url"].as_str();
    let sha256 = params["sha256"].as_str();

    if let (Some(url), Some(hash)) = (download_url, sha256) {
        match download_and_apply(url, hash, version).await {
            Ok(()) => {
                info!(version, "agent update applied, exiting for restart");
                tokio::spawn(async {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    std::process::exit(0);
                });
                Ok(serde_json::json!({ "status": "restarting", "version": version }))
            }
            Err(e) => {
                error!(error = %e, "agent update failed");
                Err(HandlerError::Other(format!("update failed: {e}")))
            }
        }
    } else {
        Ok(serde_json::json!({
            "status": "acknowledged",
            "message": "update parameters received but no download URL provided — manual update required",
        }))
    }
}

async fn download_and_apply(url: &str, expected_sha256: &str, version: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};

    let binary_path =
        std::env::current_exe().map_err(|e| format!("cannot determine binary path: {e}"))?;
    let new_path = binary_path.with_extension("new");
    let prev_path = binary_path.with_extension("prev");

    info!(url, "downloading agent update");
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("download returned HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("reading response body failed: {e}"))?;

    let hash = hex::encode(Sha256::digest(&bytes));
    if hash != expected_sha256 {
        return Err(format!(
            "SHA-256 mismatch: expected {expected_sha256}, got {hash}"
        ));
    }

    info!(size = bytes.len(), version, "download verified, applying update");

    std::fs::write(&new_path, &bytes).map_err(|e| format!("writing new binary failed: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&new_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod failed: {e}"))?;
    }

    if binary_path.exists() {
        let _ = std::fs::copy(&binary_path, &prev_path);
    }

    std::fs::rename(&new_path, &binary_path)
        .map_err(|e| format!("atomic rename failed: {e}"))?;

    info!("agent binary replaced, systemd will restart");
    Ok(())
}
