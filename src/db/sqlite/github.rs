use sqlx::{Row, SqlitePool};

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_github_installation(
    pool: &SqlitePool,
    installation_id: i64,
    account_login: &str,
    account_type: &str,
) -> Result<GitHubInstallation, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO github_installations (id, installation_id, account_login, account_type, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(installation_id)
    .bind(account_login)
    .bind(account_type)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(GitHubInstallation {
        id,
        installation_id,
        account_login: account_login.to_string(),
        account_type: account_type.to_string(),
        token_expires_at: None,
        created_at: now,
    })
}

pub(super) async fn list_github_installations(
    pool: &SqlitePool,
) -> Result<Vec<GitHubInstallation>, DbError> {
    let rows = sqlx::query(
        "SELECT id, installation_id, account_login, account_type, token_expires_at, created_at
         FROM github_installations ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| GitHubInstallation {
            id: r.get("id"),
            installation_id: r.get("installation_id"),
            account_login: r.get("account_login"),
            account_type: r.get("account_type"),
            token_expires_at: r.get("token_expires_at"),
            created_at: r.get("created_at"),
        })
        .collect())
}

pub(super) async fn delete_github_installation(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM github_installations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
