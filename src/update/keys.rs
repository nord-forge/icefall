/// A trusted Ed25519 public key for verifying release manifests. To rotate:
/// add the new key at index 0, release once signed with the old key, switch CI,
/// then drop the old key after adoption.
#[derive(Debug, Clone)]
pub struct TrustedKey {
    pub id: &'static str,
    pub fingerprint: &'static str,
    pub public_key_pem: &'static str,
    pub not_before: &'static str,
    pub not_after: Option<&'static str>,
}

pub static TRUSTED_RELEASE_KEYS: &[TrustedKey] = &[
    TrustedKey {
        id: "icefall-release-2026-r2",
        fingerprint: "sha256:81ccb1eb308387d3",
        public_key_pem: "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAO8JVuwYQ3q5YCPvXLHYRj3qaukr3/RzY6lfFVjJoRg8=\n-----END PUBLIC KEY-----",
        not_before: "2026-01-01T00:00:00Z",
        not_after: None,
    },
];

/// Returns the set of currently valid trusted keys (filters by not_before / not_after).
pub fn valid_keys(now: &str) -> Vec<&'static TrustedKey> {
    TRUSTED_RELEASE_KEYS
        .iter()
        .filter(|k| k.not_before <= now)
        .filter(|k| k.not_after.is_none_or(|expiry| now < expiry))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_key_is_present() {
        assert!(!TRUSTED_RELEASE_KEYS.is_empty());
        assert_eq!(TRUSTED_RELEASE_KEYS[0].id, "icefall-release-2026-r2");
    }

    #[test]
    fn valid_keys_filters_by_time() {
        let keys = valid_keys("2026-06-01T00:00:00Z");
        assert_eq!(keys.len(), 1);

        let keys = valid_keys("2025-01-01T00:00:00Z");
        assert_eq!(keys.len(), 0);
    }
}
