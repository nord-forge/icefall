use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::update::keys::{self, TrustedKey};
use crate::update::manifest::ReleaseManifest;

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("no valid trusted keys available")]
    NoValidKeys,
    #[error("manifest signature is invalid (no trusted key matched)")]
    InvalidSignature,
    #[error("failed to decode public key for {key_id}: {reason}")]
    KeyDecode { key_id: String, reason: String },
    #[error("failed to decode signature: {0}")]
    SignatureDecode(String),
    #[error("SHA-256 mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("version {new} is not newer than current {current}")]
    NotNewer { current: String, new: String },
    #[error("version {new} is not newer than highest seen {highest}")]
    MonotonicityViolation { highest: String, new: String },
    #[error("no artifact available for target {0}")]
    NoArtifactForTarget(String),
    #[error("semver parse error: {0}")]
    SemverParse(#[from] semver::Error),
}

/// Verify an Ed25519 signature on manifest bytes, trying each valid trusted key
/// until one succeeds. Returns the matching key ID on success.
pub fn verify_manifest_signature(
    manifest_bytes: &[u8],
    signature_b64: &str,
    now: &str,
) -> Result<String, VerifyError> {
    let valid_keys = keys::valid_keys(now);
    if valid_keys.is_empty() {
        return Err(VerifyError::NoValidKeys);
    }

    let sig_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        signature_b64.trim(),
    )
    .map_err(|e| VerifyError::SignatureDecode(e.to_string()))?;

    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| VerifyError::SignatureDecode(e.to_string()))?;

    for key in &valid_keys {
        match try_verify_with_key(manifest_bytes, &signature, key) {
            Ok(()) => return Ok(key.id.to_string()),
            Err(_) => continue,
        }
    }

    Err(VerifyError::InvalidSignature)
}

fn try_verify_with_key(
    data: &[u8],
    signature: &Signature,
    key: &TrustedKey,
) -> Result<(), VerifyError> {
    let pem_body = key
        .public_key_pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();

    let key_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &pem_body)
        .map_err(|e| VerifyError::KeyDecode {
        key_id: key.id.to_string(),
        reason: e.to_string(),
    })?;

    // Ed25519 public key in SPKI format: 12-byte header + 32-byte key
    let raw_key = if key_bytes.len() == 44 {
        &key_bytes[12..]
    } else if key_bytes.len() == 32 {
        &key_bytes[..]
    } else {
        return Err(VerifyError::KeyDecode {
            key_id: key.id.to_string(),
            reason: format!("unexpected key length: {}", key_bytes.len()),
        });
    };

    let verifying_key =
        VerifyingKey::from_bytes(raw_key.try_into().map_err(|_| VerifyError::KeyDecode {
            key_id: key.id.to_string(),
            reason: "key must be exactly 32 bytes".to_string(),
        })?)
        .map_err(|e| VerifyError::KeyDecode {
            key_id: key.id.to_string(),
            reason: e.to_string(),
        })?;

    verifying_key
        .verify(data, signature)
        .map_err(|_| VerifyError::InvalidSignature)
}

/// Compute SHA-256 hash of a byte slice and return the hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Verify that file contents match the expected SHA-256 hash.
pub fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<(), VerifyError> {
    let actual = sha256_hex(data);
    if actual != expected_hex {
        return Err(VerifyError::HashMismatch {
            expected: expected_hex.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Check that a manifest version is newer than the current running version.
pub fn check_version_newer(
    manifest: &ReleaseManifest,
    current_version: &str,
) -> Result<(), VerifyError> {
    let current = semver::Version::parse(current_version)?;
    let new = semver::Version::parse(&manifest.version)?;

    if new <= current {
        return Err(VerifyError::NotNewer {
            current: current_version.to_string(),
            new: manifest.version.clone(),
        });
    }

    Ok(())
}

/// Enforce version monotonicity: the new version must be strictly greater
/// than the highest version ever seen by this installation.
pub fn check_monotonicity(
    manifest: &ReleaseManifest,
    highest_seen: &str,
) -> Result<(), VerifyError> {
    let highest = semver::Version::parse(highest_seen)?;
    let new = semver::Version::parse(&manifest.version)?;

    if new <= highest {
        return Err(VerifyError::MonotonicityViolation {
            highest: highest_seen.to_string(),
            new: manifest.version.clone(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_works() {
        let hash = sha256_hex(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn verify_sha256_match() {
        let data = b"test data";
        let hash = sha256_hex(data);
        assert!(verify_sha256(data, &hash).is_ok());
    }

    #[test]
    fn verify_sha256_mismatch() {
        let err = verify_sha256(b"test data", "0000000000000000").unwrap_err();
        assert!(matches!(err, VerifyError::HashMismatch { .. }));
    }

    #[test]
    fn version_newer_check() {
        let manifest = crate::update::manifest::ReleaseManifest {
            schema_version: 1,
            version: "0.2.0".into(),
            release_date: String::new(),
            min_supported_version: "0.1.0".into(),
            channel: crate::update::manifest::ReleaseChannel::Stable,
            requires_migration: false,
            breaking: false,
            breaking_changes: None,
            changelog_url: None,
            artifacts: std::collections::BTreeMap::new(),
            signed_by: String::new(),
            signature_timestamp: String::new(),
        };

        assert!(check_version_newer(&manifest, "0.1.0").is_ok());
        assert!(check_version_newer(&manifest, "0.2.0").is_err());
        assert!(check_version_newer(&manifest, "0.3.0").is_err());
    }

    #[test]
    fn monotonicity_check() {
        let manifest = crate::update::manifest::ReleaseManifest {
            schema_version: 1,
            version: "0.3.0".into(),
            release_date: String::new(),
            min_supported_version: "0.1.0".into(),
            channel: crate::update::manifest::ReleaseChannel::Stable,
            requires_migration: false,
            breaking: false,
            breaking_changes: None,
            changelog_url: None,
            artifacts: std::collections::BTreeMap::new(),
            signed_by: String::new(),
            signature_timestamp: String::new(),
        };

        assert!(check_monotonicity(&manifest, "0.2.0").is_ok());
        assert!(check_monotonicity(&manifest, "0.3.0").is_err());
        assert!(check_monotonicity(&manifest, "0.4.0").is_err());
    }

    #[test]
    fn no_valid_keys_returns_error() {
        let result = verify_manifest_signature(b"data", "c2lnbmF0dXJl", "2020-01-01T00:00:00Z");
        assert!(matches!(result, Err(VerifyError::NoValidKeys)));
    }
}
