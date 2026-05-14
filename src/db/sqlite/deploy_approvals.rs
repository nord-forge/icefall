use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_deploy_approval(
    pool: &SqlitePool,
    deploy_id: &str,
    action: &str,
    user_id: &str,
    comment: Option<&str>,
) -> Result<DeployApproval, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO deploy_approvals (id, deploy_id, action, user_id, comment, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(deploy_id)
    .bind(action)
    .bind(user_id)
    .bind(comment)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(DeployApproval {
        id,
        deploy_id: deploy_id.to_string(),
        action: action.to_string(),
        user_id: user_id.to_string(),
        comment: comment.map(String::from),
        created_at: now,
    })
}

pub(super) async fn get_deploy_approval(
    pool: &SqlitePool,
    deploy_id: &str,
) -> Result<Option<DeployApproval>, DbError> {
    let approval = sqlx::query_as::<_, DeployApproval>(
        "SELECT * FROM deploy_approvals WHERE deploy_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(deploy_id)
    .fetch_optional(pool)
    .await?;
    Ok(approval)
}
