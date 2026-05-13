use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Release manifest describing a single Icefall version.
///
/// Published alongside release artifacts on GitHub Releases.
/// Signed with Ed25519; signature verified against embedded trusted keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub version: String,
    pub release_date: String,
    pub min_supported_version: String,
    pub channel: ReleaseChannel,
    pub requires_migration: bool,
    pub breaking: bool,
    pub breaking_changes: Option<String>,
    pub changelog_url: Option<String>,
    pub artifacts: BTreeMap<String, ReleaseArtifact>,
    pub signed_by: String,
    pub signature_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseArtifact {
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Nightly,
}

impl std::fmt::Display for ReleaseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stable => write!(f, "stable"),
            Self::Beta => write!(f, "beta"),
            Self::Nightly => write!(f, "nightly"),
        }
    }
}

impl ReleaseManifest {
    /// Returns the artifact entry for the current platform, if available.
    pub fn artifact_for_target(&self, target: &str) -> Option<&ReleaseArtifact> {
        self.artifacts.get(target)
    }

    /// Canonicalize the manifest to deterministic JSON for signature verification.
    /// Uses serde_json with sorted keys (BTreeMap guarantees key order).
    pub fn canonicalize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Parse a manifest from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ReleaseManifest {
        let mut artifacts = BTreeMap::new();
        artifacts.insert(
            "x86_64-linux".to_string(),
            ReleaseArtifact {
                url: "https://example.com/icefall-v0.2.0-x86_64-linux.tar.gz".to_string(),
                sha256: "abc123".to_string(),
                size_bytes: 15_000_000,
            },
        );
        artifacts.insert(
            "aarch64-linux".to_string(),
            ReleaseArtifact {
                url: "https://example.com/icefall-v0.2.0-aarch64-linux.tar.gz".to_string(),
                sha256: "def456".to_string(),
                size_bytes: 14_500_000,
            },
        );

        ReleaseManifest {
            schema_version: 1,
            version: "0.2.0".to_string(),
            release_date: "2026-05-10T14:30:00Z".to_string(),
            min_supported_version: "0.1.0".to_string(),
            channel: ReleaseChannel::Stable,
            requires_migration: false,
            breaking: false,
            breaking_changes: None,
            changelog_url: Some(
                "https://github.com/example/icefall/releases/tag/v0.2.0".to_string(),
            ),
            artifacts,
            signed_by: "sha256:cd5a5845d0ccce1f".to_string(),
            signature_timestamp: "2026-05-10T14:30:00Z".to_string(),
        }
    }

    #[test]
    fn roundtrip_serialization() {
        let manifest = sample_manifest();
        let bytes = manifest.canonicalize().unwrap();
        let parsed = ReleaseManifest::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version, "0.2.0");
        assert_eq!(parsed.schema_version, 1);
        assert!(!parsed.breaking);
    }

    #[test]
    fn artifact_lookup() {
        let manifest = sample_manifest();
        let art = manifest.artifact_for_target("x86_64-linux").unwrap();
        assert_eq!(art.sha256, "abc123");

        assert!(manifest.artifact_for_target("windows-x64").is_none());
    }

    #[test]
    fn channel_display() {
        assert_eq!(format!("{}", ReleaseChannel::Stable), "stable");
        assert_eq!(format!("{}", ReleaseChannel::Beta), "beta");
    }

    #[test]
    fn btreemap_ensures_deterministic_key_order() {
        let manifest = sample_manifest();
        let json = String::from_utf8(manifest.canonicalize().unwrap()).unwrap();
        let aarch_pos = json.find("aarch64-linux").unwrap();
        let x86_pos = json.find("x86_64-linux").unwrap();
        assert!(
            aarch_pos < x86_pos,
            "BTreeMap should sort aarch64 before x86_64"
        );
    }
}
