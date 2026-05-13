use std::collections::HashMap;
use std::sync::LazyLock;

use tokio::sync::Mutex;

pub(super) struct PkceEntry {
    pub verifier: String,
    pub created_at: std::time::Instant,
}

pub(super) static PKCE_STORE: LazyLock<Mutex<HashMap<String, PkceEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const PKCE_TTL_SECS: u64 = 300; // 5 minutes

pub(super) async fn store_pkce(state_token: &str, verifier: &str) {
    let mut store = PKCE_STORE.lock().await;
    // Prune expired entries while we're here
    store.retain(|_, v| v.created_at.elapsed().as_secs() < PKCE_TTL_SECS);
    store.insert(
        state_token.to_string(),
        PkceEntry {
            verifier: verifier.to_string(),
            created_at: std::time::Instant::now(),
        },
    );
}

pub(super) async fn take_pkce(state_token: &str) -> Option<String> {
    let mut store = PKCE_STORE.lock().await;
    store.retain(|_, v| v.created_at.elapsed().as_secs() < PKCE_TTL_SECS);
    store.remove(state_token).map(|e| e.verifier)
}
