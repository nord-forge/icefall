//! Scheduled-deploy scheduler (IF-179): a 30s loop that fires due `scheduled`
//! deploys, marking them `missed` if they came due past the grace window.

use std::time::Duration;

use crate::api::AppState;

/// How often the scheduler scans for due deploys.
const TICK: Duration = Duration::from_secs(30);

/// How long after its scheduled time a deploy may still fire. If the daemon was
/// offline past this window the deploy is marked `missed` instead (IF-179).
const GRACE_MINUTES: i64 = 30;

pub fn spawn_deploy_scheduler(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        loop {
            ticker.tick().await;
            if let Err(e) = run_tick(&state).await {
                tracing::warn!("deploy scheduler tick failed: {e}");
            }
        }
    });
}

async fn run_tick(state: &AppState) -> Result<(), crate::db::DbError> {
    let due = state.db.list_due_scheduled_deploys().await?;
    if due.is_empty() {
        return Ok(());
    }

    let now = chrono::Utc::now();

    for deploy in due {
        // Beyond the grace window the deploy is too stale to run safely.
        let overdue_past_grace = deploy
            .scheduled_at
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|when| now.signed_duration_since(when).num_minutes() > GRACE_MINUTES)
            .unwrap_or(false);

        if overdue_past_grace {
            mark_missed(state, &deploy).await;
            continue;
        }

        fire(state, &deploy).await;
    }

    Ok(())
}

async fn mark_missed(state: &AppState, deploy: &crate::db::models::Deploy) {
    let reason = "Scheduled trigger time passed while the server was offline";
    if let Err(e) = state
        .db
        .update_deploy_status(&deploy.id, "missed", Some(reason))
        .await
    {
        tracing::error!("failed to mark deploy {} missed: {e}", deploy.id);
        return;
    }

    state.event_bus.emit(
        crate::events::EventType::DeployStatus,
        Some(&deploy.app_id),
        Some(&deploy.id),
        serde_json::json!({ "status": "missed" }),
    );

    let app_name = state
        .db
        .get_app(&deploy.app_id)
        .await
        .ok()
        .flatten()
        .map(|a| a.name)
        .unwrap_or_else(|| deploy.app_id.clone());

    crate::api::routes::notifications::emit_event(
        &state.db,
        &state.config.caddy_admin_url,
        "deploy.missed",
        Some(&deploy.app_id),
        &format!("Scheduled deploy for app '{app_name}' was missed"),
        serde_json::json!({ "app": app_name, "deploy_id": deploy.id }),
    )
    .await;
}

async fn fire(state: &AppState, deploy: &crate::db::models::Deploy) {
    // Atomically claim the deploy so overlapping ticks can't double-fire it.
    match state.db.start_scheduled_deploy(&deploy.id).await {
        Ok(true) => {}
        Ok(false) => return, // already claimed/cancelled
        Err(e) => {
            tracing::error!("failed to start scheduled deploy {}: {e}", deploy.id);
            return;
        }
    }

    let Ok(Some(app)) = state.db.get_app(&deploy.app_id).await else {
        let _ = state
            .db
            .update_deploy_status(&deploy.id, "failed", Some("App no longer exists"))
            .await;
        return;
    };

    let envs = match state.db.list_environments(&deploy.app_id).await {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("scheduler could not load environments for {}: {e}", app.id);
            let _ = state
                .db
                .update_deploy_status(&deploy.id, "failed", Some("Could not load environments"))
                .await;
            return;
        }
    };
    let Some(env) = envs.into_iter().next() else {
        let _ = state
            .db
            .update_deploy_status(&deploy.id, "failed", Some("App has no environments"))
            .await;
        return;
    };

    state.event_bus.emit(
        crate::events::EventType::DeployStatus,
        Some(&deploy.app_id),
        Some(&deploy.id),
        serde_json::json!({ "status": "pending" }),
    );

    crate::api::routes::notifications::emit_event(
        &state.db,
        &state.config.caddy_admin_url,
        "deploy.started",
        Some(&deploy.app_id),
        &format!("Scheduled deploy for app '{}' started", app.name),
        serde_json::json!({ "app": app.name, "deploy_id": deploy.id }),
    )
    .await;

    tracing::info!("firing scheduled deploy {} for app {}", deploy.id, app.name);
    crate::api::routes::deploys::trigger_deploy(
        state.clone(),
        app,
        env,
        deploy.id.clone(),
        deploy.no_cache,
    )
    .await;
}
