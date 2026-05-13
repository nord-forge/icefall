use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

use crate::update::manifest::ReleaseArtifact;
use crate::update::UpdateError;

/// Downloads and extracts update artifacts.
pub struct UpdateDownloader {
    http_client: reqwest::Client,
    updates_dir: PathBuf,
}

impl UpdateDownloader {
    pub fn new(updates_dir: PathBuf) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent(format!("icefall/{}", crate::update::CURRENT_VERSION))
            .timeout(std::time::Duration::from_secs(600)) // 10 min for large binaries
            .build()
            .expect("failed to build HTTP client");

        Self {
            http_client,
            updates_dir,
        }
    }

    /// Download the update artifact with progress callback.
    ///
    /// Returns the path to the downloaded file on success.
    pub async fn download(
        &self,
        artifact: &ReleaseArtifact,
        version: &str,
        on_progress: impl Fn(u64, u64),
    ) -> Result<PathBuf, UpdateError> {
        // Ensure updates directory exists
        tokio::fs::create_dir_all(&self.updates_dir).await?;

        // 1. Check disk space (need artifact.size_bytes * 3 for download + extraction headroom)
        let needed = artifact.size_bytes * 3;
        let available = check_available_disk_space(&self.updates_dir)?;
        if available < needed {
            return Err(UpdateError::DiskSpace { needed, available });
        }

        // 2. Clean up old .partial files
        self.cleanup_partial_files().await;

        // 3. Stream download to {version}.tar.gz.partial
        let final_path = self.updates_dir.join(format!("{version}.tar.gz"));
        let partial_path = self.updates_dir.join(format!("{version}.tar.gz.partial"));

        debug!("downloading artifact to {}", partial_path.display());

        let response = self
            .http_client
            .get(&artifact.url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| UpdateError::Download(format!("HTTP error: {e}")))?;

        // 4. Track progress via Content-Length
        let total_size = response.content_length().unwrap_or(artifact.size_bytes);

        let mut file = tokio::fs::File::create(&partial_path).await?;
        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| UpdateError::Download(format!("stream error: {e}")))?;
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;
            on_progress(downloaded, total_size);
        }

        file.flush().await?;
        drop(file);

        // 5. Rename .partial to .tar.gz on completion
        tokio::fs::rename(&partial_path, &final_path).await?;

        // 6. Verify SHA-256 against artifact.sha256
        let actual_hash = hex::encode(hasher.finalize());
        if actual_hash != artifact.sha256 {
            // Remove the corrupted download
            tokio::fs::remove_file(&final_path).await.ok();
            return Err(UpdateError::Download(format!(
                "SHA-256 mismatch: expected {}, got {actual_hash}",
                artifact.sha256
            )));
        }

        info!(
            "downloaded {} ({} bytes, SHA-256 verified)",
            final_path.display(),
            downloaded
        );

        // 7. Return path to downloaded file
        Ok(final_path)
    }

    /// Extract a downloaded tarball and validate the binary.
    ///
    /// Returns the path to the extracted binary.
    pub async fn extract_and_validate(
        &self,
        tarball_path: &Path,
        version: &str,
    ) -> Result<PathBuf, UpdateError> {
        let extract_dir = self.updates_dir.join(format!("icefall-{version}"));

        // Extract in a blocking task since flate2/tar are sync
        let tarball = tarball_path.to_path_buf();
        let dest = extract_dir.clone();

        tokio::task::spawn_blocking(move || -> Result<(), UpdateError> {
            // 1. Extract to updates_dir/icefall-{version}/
            std::fs::create_dir_all(&dest)?;

            let file = std::fs::File::open(&tarball)?;
            let gz = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(gz);
            archive.set_overwrite(true);
            archive
                .unpack(&dest)
                .map_err(|e| UpdateError::Extract(format!("tar extraction failed: {e}")))?;

            Ok(())
        })
        .await
        .map_err(|e| UpdateError::Extract(format!("spawn_blocking failed: {e}")))??;

        // 2. Verify binary exists and is executable
        let binary_path = extract_dir.join("icefall");
        if !binary_path.exists() {
            // Also check inside a nested directory (some tarballs have a top-level folder)
            let nested = find_binary_in_dir(&extract_dir).await?;
            match nested {
                Some(p) => {
                    info!("extracted binary found at {}", p.display());
                    return Ok(p);
                }
                None => {
                    return Err(UpdateError::Extract(format!(
                        "no 'icefall' binary found in extracted archive at {}",
                        extract_dir.display()
                    )));
                }
            }
        }

        // Verify it's executable (on unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = tokio::fs::metadata(&binary_path).await?;
            let perms = metadata.permissions();
            if perms.mode() & 0o111 == 0 {
                let new_perms = std::fs::Permissions::from_mode(perms.mode() | 0o755);
                tokio::fs::set_permissions(&binary_path, new_perms).await?;
            }
        }

        info!("extracted binary at {}", binary_path.display());
        Ok(binary_path)
    }

    /// Remove any leftover .partial files from previous interrupted downloads.
    async fn cleanup_partial_files(&self) {
        let Ok(mut entries) = tokio::fs::read_dir(&self.updates_dir).await else {
            return;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "partial") {
                debug!("cleaning up partial file: {}", path.display());
                tokio::fs::remove_file(&path).await.ok();
            }
        }
    }
}

/// Check available disk space at the given path.
///
/// Uses `sysinfo` / `std::fs::metadata` fallback. On failure, returns u64::MAX
/// to avoid blocking updates when we can't determine space.
fn check_available_disk_space(path: &Path) -> Result<u64, UpdateError> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    // Find the disk with the longest matching mount point
    let mut best_match: Option<(usize, u64)> = None;
    for disk in disks.iter() {
        let mount = disk.mount_point();
        if canonical.starts_with(mount) {
            let mount_len = mount.as_os_str().len();
            match &best_match {
                Some((len, _)) if mount_len <= *len => continue,
                _ => best_match = Some((mount_len, disk.available_space())),
            }
        }
    }

    match best_match {
        Some((_, available)) => Ok(available),
        None => {
            warn!(
                "could not determine disk space for {}; assuming sufficient space",
                path.display()
            );
            Ok(u64::MAX)
        }
    }
}

/// Recursively search for an `icefall` binary inside a directory (one level deep).
async fn find_binary_in_dir(dir: &Path) -> Result<Option<PathBuf>, UpdateError> {
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join("icefall");
            if candidate.exists() {
                return Ok(Some(candidate));
            }
        } else if path.file_name().is_some_and(|n| n == "icefall") {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_disk_space_does_not_panic() {
        // Should return Ok regardless of platform
        let result = check_available_disk_space(Path::new("/tmp"));
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }

    #[tokio::test]
    async fn cleanup_partial_files_on_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let downloader = UpdateDownloader::new(tmp.path().to_path_buf());
        // Should not panic on empty directory
        downloader.cleanup_partial_files().await;
    }

    #[tokio::test]
    async fn cleanup_partial_files_removes_partials() {
        let tmp = tempfile::tempdir().unwrap();
        let partial = tmp.path().join("0.2.0.tar.gz.partial");
        let real = tmp.path().join("0.1.0.tar.gz");
        tokio::fs::write(&partial, b"partial data").await.unwrap();
        tokio::fs::write(&real, b"real data").await.unwrap();

        let downloader = UpdateDownloader::new(tmp.path().to_path_buf());
        downloader.cleanup_partial_files().await;

        assert!(!partial.exists(), "partial file should be removed");
        assert!(real.exists(), "non-partial file should remain");
    }
}
