//! Short-lived, single-use 2FA challenge tokens issued after a password check, so the
//! `user_id` never leaks. In-memory storage using the `LazyLock<Mutex<HashMap>>` pattern.

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

/// Resolve a challenge token to its `user_id` WITHOUT consuming it, so a mistyped
/// code doesn't force re-login — the token is invalidated on success via [`take_challenge`].
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
