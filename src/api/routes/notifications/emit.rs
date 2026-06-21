//! Event → rules → channels dispatch pipeline (IF-167). `emit_event` is the single
//! fan-out point; failures are logged, never propagated to the caller.

use std::sync::Arc;

use crate::db::Database;

use super::dispatch::dispatch_notification;

/// Rule scope that matches every app/server — used for system-wide events
/// (server reachability, disk, backups) that aren't tied to a single app.
pub const GLOBAL_SCOPE: &str = "*";

/// Fan an event out to all subscribed notification channels. `app_id` is `Some`
/// for app-scoped events and `None` for system events; app events also match global rules.
pub async fn emit_event(
    db: &Arc<dyn Database>,
    caddy_admin_url: &str,
    event_type: &str,
    app_id: Option<&str>,
    summary: &str,
    details: serde_json::Value,
) {
    let rules = match db.get_notification_rules_by_event(event_type).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("notify: could not load rules for {event_type}: {e}");
            return;
        }
    };
    if rules.is_empty() {
        return;
    }

    let channels = match db.list_notification_channels().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("notify: could not load channels: {e}");
            return;
        }
    };

    for rule in rules {
        let in_scope = match app_id {
            Some(id) => rule.app_id == id || rule.app_id == GLOBAL_SCOPE,
            None => rule.app_id == GLOBAL_SCOPE,
        };
        if !in_scope {
            continue;
        }

        let Some(channel) = channels.iter().find(|c| c.id == rule.notification_id) else {
            continue;
        };

        if let Err(e) = dispatch_notification(
            &channel.channel_type,
            &channel.config,
            event_type,
            summary,
            &details,
            caddy_admin_url,
        )
        .await
        {
            tracing::warn!(
                "notify: {event_type} via {} channel failed: {e}",
                channel.channel_type
            );
        }
    }
}
