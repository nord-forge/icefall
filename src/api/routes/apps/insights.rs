use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::team_auth::TeamCtx;
use crate::api::AppState;

/// IF-166: list the remote branches of an app's git repository, for the
/// deploy-branch picker / autocomplete.
pub(super) async fn list_branches(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = state
        .db
        .get_app_for_team(&ctx.team_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("App '{id}' not found")))?;

    let repo = app
        .git_repo
        .as_deref()
        .filter(|r| !r.trim().is_empty())
        .ok_or_else(|| ApiError::BadRequest("App has no git repository".into()))?;

    let branches = crate::build::git::list_remote_branches(repo)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Could not list branches: {e}")))?;

    Ok(Json(serde_json::json!({
        "data": branches,
        "current": app.git_branch,
    })))
}

/// Days of inactivity before an app is flagged (IF-189).
const NO_DEPLOY_DAYS: i64 = 90;
const NO_REQUEST_DAYS: i64 = 30;

/// IF-189: list apps with no recent activity (no deploy in 90d and/or no inbound
/// request in 30d). Apps flagged `exempt_from_inactivity` are skipped.
pub(super) async fn list_inactive(
    State(state): State<AppState>,
    ctx: TeamCtx,
) -> Result<Json<serde_json::Value>, ApiError> {
    let apps = state.db.list_apps_by_team(&ctx.team_id).await?;

    let now = chrono::Utc::now();
    let no_deploy_cutoff = (now - chrono::Duration::days(NO_DEPLOY_DAYS)).to_rfc3339();
    let no_request_cutoff = (now - chrono::Duration::days(NO_REQUEST_DAYS)).to_rfc3339();

    let mut inactive = Vec::new();
    for app in apps {
        if app.exempt_from_inactivity {
            continue;
        }

        let mut reasons: Vec<&str> = Vec::new();

        let last_deploy = state
            .db
            .list_deploys(&app.id, 1)
            .await
            .ok()
            .and_then(|d| d.into_iter().next());
        let last_deploy_at = last_deploy.as_ref().map(|d| d.created_at.clone());
        match last_deploy_at {
            Some(ref ts) if ts.as_str() >= no_deploy_cutoff.as_str() => {}
            _ => reasons.push("no_recent_deploy"),
        }

        match app.last_request_at {
            Some(ref ts) if ts.as_str() >= no_request_cutoff.as_str() => {}
            _ => reasons.push("no_recent_requests"),
        }

        if !reasons.is_empty() {
            inactive.push(serde_json::json!({
                "id": app.id,
                "name": app.name,
                "last_request_at": app.last_request_at,
                "last_deploy_at": last_deploy_at,
                "ghost_mode_status": app.ghost_mode_status,
                "reasons": reasons,
            }));
        }
    }

    let count = inactive.len();
    Ok(Json(
        serde_json::json!({ "data": inactive, "count": count }),
    ))
}
