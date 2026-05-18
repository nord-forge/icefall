//! Short-lived, single-use 2FA challenge tokens.
//!
//! When a login passes the password check but the account has 2FA enabled,
//! the server issues a random *challenge token* instead of revealing the
//! `user_id`. The client presents that token (not a `user_id`) to
//! `/auth/2fa/validate`. This means:
//!
//! - the `user_id` is never exposed in a response body, a redirect URL, or
//!   browser history (audit C2, M7, L3);
//! - the 2FA-validate endpoint cannot be driven with an attacker-chosen
//!   `user_id` — only a token that proves a prior successful password check;
//! - the token is single-use and expires, so a leaked token has a tiny
//!   window and cannot be replayed.
//!
//! Storage follows the same `LazyLock<Mutex<HashMap>>` pattern as
//! `oauth::pkce` — in-memory, single-node, pruned on access. Chosen for
//! maintainability: it reuses a reviewed pattern and gives single-use + TTL
//! for free, without the "stateless token still needs a used-set" trap.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Instant;

use rand::Rng;
use tokio::sync::Mutex;

struct ChallengeEntry {
    user_id: String,
    created_at: Instant,
}

static CHALLENGE_STORE: LazyLock<Mutex<HashMap<String, ChallengeEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Challenge tokens are valid for 5 minutes — long enough to fetch a code
/// from an authenticator app, short enough to bound a leaked token's value.
const CHALLENGE_TTL_SECS: u64 = 5 * 60;

/// Issue a challenge token for `user_id`. Call this only AFTER the password
/// has been verified. Returns the opaque token to hand to the client.
pub async fn issue_challenge(user_id: &str) -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token = hex::encode(bytes);

    let mut store = CHALLENGE_STORE.lock().await;
    store.retain(|_, e| e.created_at.elapsed().as_secs() < CHALLENGE_TTL_SECS);
    store.insert(
        token.clone(),
        ChallengeEntry {
            user_id: user_id.to_string(),
            created_at: Instant::now(),
        },
    );
    token
}

/// Resolve a challenge token to its `user_id` WITHOUT consuming it, if the
/// token exists and has not expired. Used to look up the user before the
/// 2FA code is checked, so a mistyped code does not force the user to log in
/// again — the token is only invalidated on success via [`take_challenge`].
/// Brute-force is bounded separately by the per-IP rate limit on validate.
pub async fn peek_challenge(token: &str) -> Option<String> {
    let mut store = CHALLENGE_STORE.lock().await;
    store.retain(|_, e| e.created_at.elapsed().as_secs() < CHALLENGE_TTL_SECS);
    store.get(token).map(|e| e.user_id.clone())
}

/// Consume a challenge token, returning the associated `user_id` if the
/// token exists and has not expired. Single-use: the entry is removed.
pub async fn take_challenge(token: &str) -> Option<String> {
    let mut store = CHALLENGE_STORE.lock().await;
    store.retain(|_, e| e.created_at.elapsed().as_secs() < CHALLENGE_TTL_SECS);
    store.remove(token).map(|e| e.user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn issued_token_resolves_once_then_is_consumed() {
        let token = issue_challenge("user-123").await;
        assert_eq!(take_challenge(&token).await.as_deref(), Some("user-123"));
        // Single use — the second take finds nothing.
        assert_eq!(take_challenge(&token).await, None);
    }

    #[tokio::test]
    async fn unknown_token_resolves_to_none() {
        assert_eq!(take_challenge("not-a-real-token").await, None);
    }

    #[tokio::test]
    async fn distinct_issues_produce_distinct_tokens() {
        let a = issue_challenge("user-a").await;
        let b = issue_challenge("user-b").await;
        assert_ne!(a, b);
        assert_eq!(take_challenge(&a).await.as_deref(), Some("user-a"));
        assert_eq!(take_challenge(&b).await.as_deref(), Some("user-b"));
    }
}
