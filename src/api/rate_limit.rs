//! In-memory sliding-window rate limiter (per-IP on auth, per-user on the API,
//! never trusting `X-Forwarded-For`). Uses the same `LazyLock<Mutex<HashMap>>` pattern as `oauth::pkce`.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use tokio::sync::Mutex;

/// A single rate-limit bucket: the timestamps of recent hits for one key.
struct Bucket {
    hits: Vec<Instant>,
}

/// A named limiter with a fixed quota and window. Each distinct `Quota`
/// (login, register, api, global) gets its own store so keys never collide.
pub struct Quota {
    store: LazyLock<Mutex<HashMap<String, Bucket>>>,
    max: usize,
    window: Duration,
}

impl Quota {
    const fn new(max: usize, window: Duration) -> Self {
        Self {
            store: LazyLock::new(|| Mutex::new(HashMap::new())),
            max,
            window,
        }
    }

    /// Record a hit for `key`. Returns `true` if the request is allowed,
    /// `false` if the quota for this window is exhausted.
    pub async fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut store = self.store.lock().await;

        // Drop fully-expired buckets so the map cannot grow unbounded
        // (this is the eviction the audit's M9 cares about, applied here).
        store.retain(|_, b| b.hits.iter().any(|t| now.duration_since(*t) < self.window));

        let bucket = store
            .entry(key.to_string())
            .or_insert_with(|| Bucket { hits: Vec::new() });
        bucket.hits.retain(|t| now.duration_since(*t) < self.window);

        if bucket.hits.len() >= self.max {
            return false;
        }
        bucket.hits.push(now);
        true
    }
}

/// 5 login attempts / 15 min, per IP. (`rules/security.md`)
pub static LOGIN: Quota = Quota::new(5, Duration::from_secs(15 * 60));
/// 3 registrations / hour, per IP.
pub static REGISTER: Quota = Quota::new(3, Duration::from_secs(60 * 60));
/// 5 2FA validations / 5 min, per IP (audit C2 recommends 5 / 5 min).
pub static TWO_FACTOR: Quota = Quota::new(5, Duration::from_secs(5 * 60));
/// Global per-IP safety net: 1000 req / 15 min.
pub static GLOBAL: Quota = Quota::new(1000, Duration::from_secs(15 * 60));
/// Authenticated API: 100 req / 15 min, per user. (Used in `require_auth`.)
pub static API_PER_USER: Quota = Quota::new(100, Duration::from_secs(15 * 60));

/// Extract the trusted client IP from proxy headers: trust `cf-connecting-ip` and
/// `x-real-ip`, never spoofable `x-forwarded-for`. Falls back to a shared bucket.
pub fn client_ip(headers: &HeaderMap) -> String {
    for name in ["cf-connecting-ip", "x-real-ip"] {
        if let Some(v) = headers.get(name).and_then(|v| v.to_str().ok()) {
            let v = v.trim();
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn quota_allows_up_to_max_then_blocks() {
        static Q: Quota = Quota::new(3, Duration::from_secs(60));
        let key = "quota_allows_up_to_max_then_blocks";
        assert!(Q.check(key).await);
        assert!(Q.check(key).await);
        assert!(Q.check(key).await);
        // 4th hit exceeds the quota of 3.
        assert!(!Q.check(key).await);
    }

    #[tokio::test]
    async fn quota_keys_are_independent() {
        static Q: Quota = Quota::new(1, Duration::from_secs(60));
        assert!(Q.check("key-a").await);
        assert!(!Q.check("key-a").await);
        // A different key has its own bucket.
        assert!(Q.check("key-b").await);
    }

    #[tokio::test]
    async fn expired_hits_are_dropped() {
        static Q: Quota = Quota::new(1, Duration::from_millis(50));
        let key = "expired_hits_are_dropped";
        assert!(Q.check(key).await);
        assert!(!Q.check(key).await);
        tokio::time::sleep(Duration::from_millis(70)).await;
        // Window elapsed — the bucket is empty again.
        assert!(Q.check(key).await);
    }

    #[test]
    fn client_ip_prefers_cf_then_real_ip_never_xff() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
        // X-Forwarded-For is never trusted — falls through to "unknown".
        assert_eq!(client_ip(&h), "unknown");

        h.insert("x-real-ip", "5.6.7.8".parse().unwrap());
        assert_eq!(client_ip(&h), "5.6.7.8");

        h.insert("cf-connecting-ip", "9.9.9.9".parse().unwrap());
        assert_eq!(client_ip(&h), "9.9.9.9");
    }
}
