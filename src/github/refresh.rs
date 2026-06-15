//! GitHub installation-token refresh scheduler (IF-174).
//!
//! Installation tokens expire after ~1 hour. Every 30 minutes this loop mints
//! fresh tokens for installations whose cached token is missing or expires
//! within the next ~35 minutes, so a token is always warm when a deploy needs to
//! report status or comment on a PR.

use std::sync::Arc;
use std::time::Duration;

use crate::db::Database;
use crate::github::auth::generate_jwt;
use crate::github::client::GitHubClient;

/// How often the refresh loop runs.
const TICK: Duration = Duration::from_secs(30 * 60);

/// Refresh tokens expiring within this window on each tick (covers the 30-min
/// interval plus margin).
const REFRESH_AHEAD_MINUTES: i64 = 35;

pub fn spawn_token_refresher(db: Arc<dyn Database>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        loop {
            ticker.tick().await;
            if let Err(e) = run_tick(&db).await {
                tracing::warn!("GitHub token refresh tick failed: {e}");
            }
        }
    });
}

async fn run_tick(db: &Arc<dyn Database>) -> Result<(), crate::db::DbError> {
    let threshold = (chrono::Utc::now() + chrono::Duration::minutes(REFRESH_AHEAD_MINUTES))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let due = db
        .list_installations_needing_token_refresh(&threshold)
        .await?;
    if due.is_empty() {
        return Ok(());
    }

    for installation in due {
        // Each needs the App's private key to mint a token.
        let app = match db
            .get_github_app_for_installation(installation.installation_id)
            .await
        {
            Ok(Some(app)) => app,
            Ok(None) => continue, // Unlinked installation — nothing to refresh.
            Err(e) => {
                tracing::warn!(
                    installation_id = installation.installation_id,
                    error = %e,
                    "failed to load app for token refresh"
                );
                continue;
            }
        };

        let jwt = match generate_jwt(app.app_id, &app.private_key) {
            Ok(jwt) => jwt,
            Err(e) => {
                tracing::warn!(app_id = app.app_id, error = %e, "JWT generation failed during refresh");
                continue;
            }
        };

        let client = GitHubClient::new(&app.api_url);
        match client
            .get_installation_token(&jwt, installation.installation_id)
            .await
        {
            Ok(token) => {
                if let Err(e) = db
                    .update_github_installation_token(
                        installation.installation_id,
                        &token.token,
                        &token.expires_at,
                    )
                    .await
                {
                    tracing::warn!(
                        installation_id = installation.installation_id,
                        error = %e,
                        "failed to store refreshed token"
                    );
                } else {
                    tracing::debug!(
                        installation_id = installation.installation_id,
                        "refreshed GitHub installation token"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    installation_id = installation.installation_id,
                    error = %e,
                    "failed to refresh installation token"
                );
            }
        }
    }

    Ok(())
}
