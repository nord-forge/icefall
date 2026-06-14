use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_deploy(
    pool: &SqlitePool,
    deploy: &NewDeploy,
) -> Result<Deploy, DbError> {
    let id = new_id();
    let now = now_iso8601();

    // A scheduled deploy waits in the 'scheduled' state with no start time until
    // the scheduler fires it; an immediate deploy starts 'pending' right away.
    let (status, started_at) = match deploy.scheduled_at {
        Some(_) => ("scheduled", None),
        None => ("pending", Some(now.as_str())),
    };

    sqlx::query(
        "INSERT INTO deploys (id, app_id, environment_id, status, git_sha, server_id, tag, no_cache, scheduled_at, started_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&deploy.app_id)
    .bind(&deploy.environment_id)
    .bind(status)
    .bind(&deploy.git_sha)
    .bind(&deploy.server_id)
    .bind(&deploy.tag)
    .bind(deploy.no_cache)
    .bind(&deploy.scheduled_at)
    .bind(started_at)
    .bind(&now)
    .execute(pool)
    .await?;

    get_deploy(pool, &id)
        .await?
        .ok_or_else(|| DbError::NotFound(id))
}

/// Scheduled deploys whose trigger time has arrived (`scheduled_at <= now`),
/// oldest first. The scheduler decides per-deploy whether to fire or mark it
/// missed (IF-179).
pub(super) async fn list_due_scheduled_deploys(pool: &SqlitePool) -> Result<Vec<Deploy>, DbError> {
    let now = now_iso8601();
    let deploys = sqlx::query_as::<_, Deploy>(
        "SELECT * FROM deploys WHERE status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= ? ORDER BY scheduled_at ASC",
    )
    .bind(&now)
    .fetch_all(pool)
    .await?;
    Ok(deploys)
}

/// Move a scheduled deploy into the active pipeline: 'pending' with a start
/// time stamped. Returns whether a row actually transitioned, so concurrent
/// scheduler ticks don't double-fire the same deploy.
pub(super) async fn start_scheduled_deploy(
    pool: &SqlitePool,
    deploy_id: &str,
) -> Result<bool, DbError> {
    let now = now_iso8601();
    let res = sqlx::query(
        "UPDATE deploys SET status = 'pending', started_at = ? WHERE id = ? AND status = 'scheduled'",
    )
    .bind(&now)
    .bind(deploy_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Change the trigger time of a still-scheduled deploy (reschedule, IF-179).
/// Returns whether the deploy was still in the 'scheduled' state.
pub(super) async fn reschedule_deploy(
    pool: &SqlitePool,
    deploy_id: &str,
    scheduled_at: &str,
) -> Result<bool, DbError> {
    let res =
        sqlx::query("UPDATE deploys SET scheduled_at = ? WHERE id = ? AND status = 'scheduled'")
            .bind(scheduled_at)
            .bind(deploy_id)
            .execute(pool)
            .await?;
    Ok(res.rows_affected() > 0)
}

pub(super) async fn get_deploy(pool: &SqlitePool, id: &str) -> Result<Option<Deploy>, DbError> {
    let deploy = sqlx::query_as::<_, Deploy>("SELECT * FROM deploys WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(deploy)
}

pub(super) async fn list_deploys(
    pool: &SqlitePool,
    app_id: &str,
    limit: i64,
) -> Result<Vec<Deploy>, DbError> {
    let deploys = sqlx::query_as::<_, Deploy>(
        "SELECT * FROM deploys WHERE app_id = ? ORDER BY created_at DESC LIMIT ?",
    )
    .bind(app_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(deploys)
}

pub(super) async fn get_latest_deploys_for_apps(
    pool: &SqlitePool,
    app_ids: &[String],
) -> Result<Vec<Deploy>, DbError> {
    if app_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: Vec<String> = app_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect();
    let query = format!(
        "SELECT d.* FROM deploys d
         INNER JOIN (
           SELECT app_id, MAX(created_at) as max_created
           FROM deploys
           GROUP BY app_id
         ) latest ON d.app_id = latest.app_id AND d.created_at = latest.max_created
         WHERE d.app_id IN ({})",
        placeholders.join(", ")
    );
    let mut q = sqlx::query_as::<_, Deploy>(&query);
    for id in app_ids {
        q = q.bind(id);
    }
    let deploys = q.fetch_all(pool).await?;
    Ok(deploys)
}

pub(super) async fn update_deploy_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    log: Option<&str>,
) -> Result<(), DbError> {
    let now = now_iso8601();

    sqlx::query(
        "UPDATE deploys SET status = ?, build_log = COALESCE(?, build_log), finished_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(log)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

// --- Deploy extras ---

pub(super) async fn update_deploy_container_id(
    pool: &SqlitePool,
    deploy_id: &str,
    container_id: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE deploys SET container_id = ? WHERE id = ?")
        .bind(container_id)
        .bind(deploy_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_deploy_image_ref(
    pool: &SqlitePool,
    deploy_id: &str,
    image_ref: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE deploys SET image_ref = ? WHERE id = ?")
        .bind(image_ref)
        .bind(deploy_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_deploy_env_snapshot(
    pool: &SqlitePool,
    deploy_id: &str,
    env_snapshot: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE deploys SET env_snapshot = ? WHERE id = ?")
        .bind(env_snapshot)
        .bind(deploy_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_deploy_config_hash(
    pool: &SqlitePool,
    deploy_id: &str,
    config_hash: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE deploys SET config_hash = ? WHERE id = ?")
        .bind(config_hash)
        .bind(deploy_id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- Lookup helpers ---

pub(super) async fn get_app_by_repo(
    pool: &SqlitePool,
    repo_url: &str,
) -> Result<Option<App>, DbError> {
    let apps = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE git_repo IS NOT NULL")
        .fetch_all(pool)
        .await?;
    let normalized = super::normalize_repo_url(repo_url);
    Ok(apps
        .into_iter()
        .find(|a| a.git_repo.as_deref().map(super::normalize_repo_url) == Some(normalized.clone())))
}

pub(super) async fn has_active_deploys(pool: &SqlitePool) -> Result<bool, DbError> {
    let row =
        sqlx::query("SELECT id FROM deploys WHERE status IN ('building', 'deploying') LIMIT 1")
            .fetch_optional(pool)
            .await?;
    Ok(row.is_some())
}
