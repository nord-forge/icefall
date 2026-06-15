//! Installation-token resolution (IF-174): return a valid GitHub App
//! installation token, minting a fresh one via the App JWT when the cached token
//! is missing or near expiry, and caching the result (encrypted) in the DB.

use std::sync::Arc;

use crate::db::Database;
use crate::github::auth::generate_jwt;
use crate::github::client::GitHubClient;

/// Refresh the cached token when it expires within this window.
const REFRESH_SKEW_SECS: i64 = 5 * 60;

/// A token resolved for an installation, with the API base URL to use with it.
pub struct ResolvedToken {
    pub token: String,
    pub api_url: String,
    pub account_login: String,
    /// GitHub's numeric installation id (for comment/webhook tracking rows).
    pub installation_id: i64,
}

/// Return a usable installation token for `installation_db_id` (the DB id, not
/// the GitHub numeric id). Uses the cached token when still valid, otherwise
/// mints and caches a new one.
pub async fn get_valid_installation_token(
    db: &Arc<dyn Database>,
    installation_db_id: &str,
) -> Result<ResolvedToken, String> {
    let installation = db
        .get_github_installation(installation_db_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("installation {installation_db_id} not found"))?;

    let app = db
        .get_github_app_for_installation(installation.installation_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "installation is not linked to a GitHub App".to_string())?;

    // Use the cached token when it is present and not within the refresh skew.
    if let (Some(token), Some(expires_at)) =
        (&installation.access_token, &installation.token_expires_at)
    {
        if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires_at) {
            let remaining = exp.timestamp() - chrono::Utc::now().timestamp();
            if remaining > REFRESH_SKEW_SECS {
                return Ok(ResolvedToken {
                    token: token.clone(),
                    api_url: app.api_url.clone(),
                    account_login: installation.account_login.clone(),
                    installation_id: installation.installation_id,
                });
            }
        }
    }

    // Mint a fresh installation token via the App JWT and cache it.
    let jwt = generate_jwt(app.app_id, &app.private_key)?;
    let client = GitHubClient::new(&app.api_url);
    let minted = client
        .get_installation_token(&jwt, installation.installation_id)
        .await?;

    db.update_github_installation_token(
        installation.installation_id,
        &minted.token,
        &minted.expires_at,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(ResolvedToken {
        token: minted.token,
        api_url: app.api_url,
        account_login: installation.account_login,
        installation_id: installation.installation_id,
    })
}
