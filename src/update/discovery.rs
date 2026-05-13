use serde::Deserialize;
use tracing::{debug, warn};

use crate::update::manifest::ReleaseManifest;
use crate::update::verify;
use crate::update::UpdateError;

/// Metadata extracted from a GitHub release, complementing the verified manifest.
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub release_url: String,
    pub release_notes: String,
    pub changelog_highlights: Vec<String>,
    pub published_at: String,
}

/// Checks GitHub Releases for newer Icefall versions.
pub struct UpdateChecker {
    http_client: reqwest::Client,
    github_repo: String,
}

/// Subset of the GitHub Releases API response we need.
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

impl UpdateChecker {
    pub fn new(github_repo: &str) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent(format!("icefall/{}", crate::update::CURRENT_VERSION))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            http_client,
            github_repo: github_repo.to_string(),
        }
    }

    /// Check GitHub Releases for a newer version.
    ///
    /// Returns `Some((manifest, info))` if an update is available, `None` if current.
    pub async fn check_for_update(
        &self,
        current_version: &str,
        channel: &str,
        highest_seen: &str,
    ) -> Result<Option<(ReleaseManifest, ReleaseInfo)>, UpdateError> {
        // 1. Fetch releases from GitHub API
        let releases = self.fetch_releases().await?;

        // 2. Find the latest matching release (filter by channel / prerelease flag)
        let Some(release) = Self::find_matching_release(&releases, channel) else {
            debug!("no matching release found for channel '{channel}'");
            return Ok(None);
        };

        debug!(
            "found release {} (prerelease={})",
            release.tag_name, release.prerelease
        );

        // 3. Download manifest JSON from release assets
        let Some(manifest_url) = Self::find_asset(&release.assets, "manifest.json") else {
            warn!("release {} has no manifest.json asset", release.tag_name);
            return Ok(None);
        };

        let manifest_bytes = self
            .http_client
            .get(&manifest_url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let manifest = ReleaseManifest::from_bytes(&manifest_bytes)?;

        // 4. Download manifest signature from release assets
        let Some(signature_url) = Self::find_asset(&release.assets, "manifest.json.sig") else {
            warn!(
                "release {} has no manifest.json.sig asset",
                release.tag_name
            );
            return Ok(None);
        };

        let signature_b64 = self
            .http_client
            .get(&signature_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // 5. Verify signature against trusted keys
        let now = crate::db::models::now_iso8601();
        verify::verify_manifest_signature(&manifest_bytes, &signature_b64, &now)?;

        // 6. Check version is newer than current AND highest_seen
        if verify::check_version_newer(&manifest, current_version).is_err() {
            debug!(
                "release {} is not newer than current {current_version}",
                manifest.version
            );
            return Ok(None);
        }

        if verify::check_monotonicity(&manifest, highest_seen).is_err() {
            debug!(
                "release {} does not pass monotonicity check against {highest_seen}",
                manifest.version
            );
            return Ok(None);
        }

        // 7. Parse changelog highlights from release body (first 5 bullet items)
        let release_notes = release.body.clone().unwrap_or_default();
        let highlights = Self::parse_changelog_highlights(&release_notes);

        // 8. Return manifest + release info
        let info = ReleaseInfo {
            version: manifest.version.clone(),
            release_url: release.html_url.clone(),
            release_notes,
            changelog_highlights: highlights,
            published_at: release.published_at.clone().unwrap_or_default(),
        };

        Ok(Some((manifest, info)))
    }

    /// Fetch the latest releases from the GitHub API.
    async fn fetch_releases(&self) -> Result<Vec<GitHubRelease>, UpdateError> {
        let url = format!(
            "https://api.github.com/repos/{}/releases?per_page=10",
            self.github_repo
        );

        let response = self
            .http_client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?;

        let releases: Vec<GitHubRelease> = response.json().await?;
        Ok(releases)
    }

    /// Find the first release matching the channel criteria.
    ///
    /// - `stable` channel: non-prerelease only
    /// - `beta` channel: prerelease or stable
    /// - `nightly` channel: any release
    fn find_matching_release<'a>(
        releases: &'a [GitHubRelease],
        channel: &str,
    ) -> Option<&'a GitHubRelease> {
        releases.iter().find(|r| match channel {
            "stable" => !r.prerelease,
            "beta" => true, // beta users see both prerelease and stable
            "nightly" => true,
            _ => !r.prerelease,
        })
    }

    /// Find an asset by filename in the release assets list.
    fn find_asset(assets: &[GitHubAsset], filename: &str) -> Option<String> {
        assets
            .iter()
            .find(|a| a.name == filename)
            .map(|a| a.browser_download_url.clone())
    }

    /// Extract the first 5 markdown bullet points from the release body.
    fn parse_changelog_highlights(body: &str) -> Vec<String> {
        body.lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    Some(trimmed[2..].trim().to_string())
                } else {
                    None
                }
            })
            .take(5)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_changelog_highlights_extracts_bullets() {
        let body = "## What's new\n\n- Feature A added\n- Bug B fixed\n* Improved C\nSome text\n- D\n- E\n- F should be excluded";
        let highlights = UpdateChecker::parse_changelog_highlights(body);
        assert_eq!(highlights.len(), 5);
        assert_eq!(highlights[0], "Feature A added");
        assert_eq!(highlights[1], "Bug B fixed");
        assert_eq!(highlights[2], "Improved C");
        assert_eq!(highlights[3], "D");
        assert_eq!(highlights[4], "E");
    }

    #[test]
    fn parse_changelog_highlights_empty_body() {
        let highlights = UpdateChecker::parse_changelog_highlights("");
        assert!(highlights.is_empty());
    }

    #[test]
    fn parse_changelog_highlights_no_bullets() {
        let highlights =
            UpdateChecker::parse_changelog_highlights("Just a paragraph\nwith no bullets.");
        assert!(highlights.is_empty());
    }

    #[test]
    fn find_matching_release_stable_channel() {
        let releases = vec![
            GitHubRelease {
                tag_name: "v0.3.0-beta.1".into(),
                html_url: "https://example.com".into(),
                body: None,
                prerelease: true,
                published_at: None,
                assets: vec![],
            },
            GitHubRelease {
                tag_name: "v0.2.0".into(),
                html_url: "https://example.com".into(),
                body: None,
                prerelease: false,
                published_at: None,
                assets: vec![],
            },
        ];

        let result = UpdateChecker::find_matching_release(&releases, "stable");
        assert!(result.is_some());
        assert_eq!(result.unwrap().tag_name, "v0.2.0");
    }

    #[test]
    fn find_matching_release_beta_channel() {
        let releases = vec![
            GitHubRelease {
                tag_name: "v0.3.0-beta.1".into(),
                html_url: "https://example.com".into(),
                body: None,
                prerelease: true,
                published_at: None,
                assets: vec![],
            },
            GitHubRelease {
                tag_name: "v0.2.0".into(),
                html_url: "https://example.com".into(),
                body: None,
                prerelease: false,
                published_at: None,
                assets: vec![],
            },
        ];

        // Beta channel sees prereleases first
        let result = UpdateChecker::find_matching_release(&releases, "beta");
        assert!(result.is_some());
        assert_eq!(result.unwrap().tag_name, "v0.3.0-beta.1");
    }

    #[test]
    fn find_asset_by_name() {
        let assets = vec![
            GitHubAsset {
                name: "icefall-v0.2.0-x86_64-linux.tar.gz".into(),
                browser_download_url: "https://example.com/tarball".into(),
            },
            GitHubAsset {
                name: "manifest.json".into(),
                browser_download_url: "https://example.com/manifest".into(),
            },
            GitHubAsset {
                name: "manifest.json.sig".into(),
                browser_download_url: "https://example.com/sig".into(),
            },
        ];

        assert_eq!(
            UpdateChecker::find_asset(&assets, "manifest.json"),
            Some("https://example.com/manifest".into())
        );
        assert_eq!(
            UpdateChecker::find_asset(&assets, "manifest.json.sig"),
            Some("https://example.com/sig".into())
        );
        assert_eq!(UpdateChecker::find_asset(&assets, "missing.txt"), None);
    }
}
