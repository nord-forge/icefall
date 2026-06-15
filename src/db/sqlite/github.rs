use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

// --- GitHub Installations ---

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
        access_token: None,
        token_expires_at: None,
        github_app_id: None,
        created_at: now,
    })
}

pub(super) async fn list_github_installations(
    pool: &SqlitePool,
) -> Result<Vec<GitHubInstallation>, DbError> {
    let rows = sqlx::query(
        "SELECT id, installation_id, account_login, account_type, token_expires_at, github_app_id, created_at
         FROM github_installations ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    // Bulk metadata listing — never decrypts the access token.
    Ok(rows
        .into_iter()
        .map(|r| GitHubInstallation {
            id: r.get("id"),
            installation_id: r.get("installation_id"),
            account_login: r.get("account_login"),
            account_type: r.get("account_type"),
            access_token: None,
            token_expires_at: r.get("token_expires_at"),
            github_app_id: r.get("github_app_id"),
            created_at: r.get("created_at"),
        })
        .collect())
}

/// Fetch one installation by its DB id, decrypting the cached access token.
pub(super) async fn get_github_installation(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    id: &str,
) -> Result<Option<GitHubInstallation>, DbError> {
    let row = sqlx::query(
        "SELECT id, installation_id, account_login, account_type, access_token_encrypted, token_expires_at, github_app_id, created_at
         FROM github_installations WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(decrypt_installation_row(&r, encryptor)?)),
        None => Ok(None),
    }
}

/// Fetch one installation by its GitHub installation_id, decrypting the token.
pub(super) async fn get_github_installation_by_installation_id(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    installation_id: i64,
) -> Result<Option<GitHubInstallation>, DbError> {
    let row = sqlx::query(
        "SELECT id, installation_id, account_login, account_type, access_token_encrypted, token_expires_at, github_app_id, created_at
         FROM github_installations WHERE installation_id = ?",
    )
    .bind(installation_id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(decrypt_installation_row(&r, encryptor)?)),
        None => Ok(None),
    }
}

/// Store a freshly-minted installation access token (encrypted) and its expiry.
pub(super) async fn update_github_installation_token(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    installation_id: i64,
    access_token: &str,
    token_expires_at: &str,
) -> Result<(), DbError> {
    let encrypted = encryptor.encrypt(access_token.as_bytes())?;
    sqlx::query(
        "UPDATE github_installations SET access_token_encrypted = ?, token_expires_at = ?
         WHERE installation_id = ?",
    )
    .bind(&encrypted)
    .bind(token_expires_at)
    .bind(installation_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Installations whose cached token is missing or expires before `threshold`
/// (RFC3339). Used by the refresh background task.
pub(super) async fn list_installations_needing_token_refresh(
    pool: &SqlitePool,
    threshold: &str,
) -> Result<Vec<GitHubInstallation>, DbError> {
    let rows = sqlx::query(
        "SELECT id, installation_id, account_login, account_type, token_expires_at, github_app_id, created_at
         FROM github_installations
         WHERE github_app_id IS NOT NULL
           AND (token_expires_at IS NULL OR token_expires_at < ?)
         ORDER BY created_at ASC",
    )
    .bind(threshold)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| GitHubInstallation {
            id: r.get("id"),
            installation_id: r.get("installation_id"),
            account_login: r.get("account_login"),
            account_type: r.get("account_type"),
            access_token: None,
            token_expires_at: r.get("token_expires_at"),
            github_app_id: r.get("github_app_id"),
            created_at: r.get("created_at"),
        })
        .collect())
}

fn decrypt_installation_row(
    r: &sqlx::sqlite::SqliteRow,
    encryptor: &Encryptor,
) -> Result<GitHubInstallation, DbError> {
    let encrypted: Option<Vec<u8>> = r.get("access_token_encrypted");
    let access_token = match encrypted {
        Some(bytes) if !bytes.is_empty() => {
            Some(String::from_utf8(encryptor.decrypt(&bytes)?).unwrap_or_default())
        }
        _ => None,
    };
    Ok(GitHubInstallation {
        id: r.get("id"),
        installation_id: r.get("installation_id"),
        account_login: r.get("account_login"),
        account_type: r.get("account_type"),
        access_token,
        token_expires_at: r.get("token_expires_at"),
        github_app_id: r.get("github_app_id"),
        created_at: r.get("created_at"),
    })
}

pub(super) async fn delete_github_installation(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM github_installations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- GitHub Apps ---

pub(super) async fn create_github_app(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    app: &GitHubApp,
) -> Result<GitHubApp, DbError> {
    let client_secret_encrypted = encryptor.encrypt(app.client_secret.as_bytes())?;
    let private_key_encrypted = encryptor.encrypt(app.private_key.as_bytes())?;
    let webhook_secret_encrypted = encryptor.encrypt(app.webhook_secret.as_bytes())?;

    sqlx::query(
        "INSERT INTO github_apps (id, name, app_id, client_id, client_secret_encrypted, private_key_encrypted, webhook_secret_encrypted, html_url, api_url, owner_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&app.id)
    .bind(&app.name)
    .bind(app.app_id)
    .bind(&app.client_id)
    .bind(&client_secret_encrypted)
    .bind(&private_key_encrypted)
    .bind(&webhook_secret_encrypted)
    .bind(&app.html_url)
    .bind(&app.api_url)
    .bind(&app.owner_id)
    .bind(&app.created_at)
    .bind(&app.updated_at)
    .execute(pool)
    .await?;

    Ok(app.clone())
}

pub(super) async fn get_github_app(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    id: &str,
) -> Result<Option<GitHubApp>, DbError> {
    let row = sqlx::query(
        "SELECT id, name, app_id, client_id, client_secret_encrypted, private_key_encrypted, webhook_secret_encrypted, html_url, api_url, owner_id, created_at, updated_at
         FROM github_apps WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(decrypt_github_app_row(&r, encryptor)?)),
        None => Ok(None),
    }
}

pub(super) async fn list_github_apps(
    pool: &SqlitePool,
    encryptor: &Encryptor,
) -> Result<Vec<GitHubApp>, DbError> {
    let rows = sqlx::query(
        "SELECT id, name, app_id, client_id, client_secret_encrypted, private_key_encrypted, webhook_secret_encrypted, html_url, api_url, owner_id, created_at, updated_at
         FROM github_apps ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let mut apps = Vec::with_capacity(rows.len());
    for r in rows {
        apps.push(decrypt_github_app_row(&r, encryptor)?);
    }
    Ok(apps)
}

pub(super) async fn delete_github_app(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    // Unlink any installations first
    sqlx::query("UPDATE github_installations SET github_app_id = NULL WHERE github_app_id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM github_apps WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_github_installation_app_id(
    pool: &SqlitePool,
    installation_id: i64,
    github_app_id: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE github_installations SET github_app_id = ? WHERE installation_id = ?")
        .bind(github_app_id)
        .bind(installation_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn get_github_app_for_installation(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    installation_id: i64,
) -> Result<Option<GitHubApp>, DbError> {
    let row = sqlx::query(
        "SELECT ga.id, ga.name, ga.app_id, ga.client_id, ga.client_secret_encrypted, ga.private_key_encrypted, ga.webhook_secret_encrypted, ga.html_url, ga.api_url, ga.owner_id, ga.created_at, ga.updated_at
         FROM github_apps ga
         INNER JOIN github_installations gi ON gi.github_app_id = ga.id
         WHERE gi.installation_id = ?",
    )
    .bind(installation_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(decrypt_github_app_row(&r, encryptor)?)),
        None => Ok(None),
    }
}

// --- GitHub PR comments (preview-env status) ---

/// The tracked comment for an (app, PR), if Icefall has posted one.
pub(super) async fn get_github_pr_comment(
    pool: &SqlitePool,
    app_id: &str,
    pr_number: i64,
) -> Result<Option<GitHubPrComment>, DbError> {
    let row = sqlx::query_as::<_, (String, String, i64, String, i64, i64, String, String)>(
        "SELECT id, app_id, installation_id, repo_full_name, pr_number, comment_id, created_at, updated_at
         FROM github_pr_comments WHERE app_id = ? AND pr_number = ?",
    )
    .bind(app_id)
    .bind(pr_number)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| GitHubPrComment {
        id: r.0,
        app_id: r.1,
        installation_id: r.2,
        repo_full_name: r.3,
        pr_number: r.4,
        comment_id: r.5,
        created_at: r.6,
        updated_at: r.7,
    }))
}

/// Record a newly-posted PR comment (upsert on the (app, PR) pair).
pub(super) async fn upsert_github_pr_comment(
    pool: &SqlitePool,
    app_id: &str,
    installation_id: i64,
    repo_full_name: &str,
    pr_number: i64,
    comment_id: i64,
) -> Result<(), DbError> {
    let id = new_id();
    let now = now_iso8601();
    sqlx::query(
        "INSERT INTO github_pr_comments
            (id, app_id, installation_id, repo_full_name, pr_number, comment_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(app_id, pr_number) DO UPDATE SET
            comment_id = excluded.comment_id,
            installation_id = excluded.installation_id,
            repo_full_name = excluded.repo_full_name,
            updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(app_id)
    .bind(installation_id)
    .bind(repo_full_name)
    .bind(pr_number)
    .bind(comment_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

fn decrypt_github_app_row(
    r: &sqlx::sqlite::SqliteRow,
    encryptor: &Encryptor,
) -> Result<GitHubApp, DbError> {
    let client_secret_encrypted: Vec<u8> = r.get("client_secret_encrypted");
    let private_key_encrypted: Vec<u8> = r.get("private_key_encrypted");
    let webhook_secret_encrypted: Vec<u8> = r.get("webhook_secret_encrypted");

    let client_secret =
        String::from_utf8(encryptor.decrypt(&client_secret_encrypted)?).unwrap_or_default();
    let private_key =
        String::from_utf8(encryptor.decrypt(&private_key_encrypted)?).unwrap_or_default();
    let webhook_secret =
        String::from_utf8(encryptor.decrypt(&webhook_secret_encrypted)?).unwrap_or_default();

    Ok(GitHubApp {
        id: r.get("id"),
        name: r.get("name"),
        app_id: r.get("app_id"),
        client_id: r.get("client_id"),
        client_secret,
        private_key,
        webhook_secret,
        html_url: r.get("html_url"),
        api_url: r.get("api_url"),
        owner_id: r.get("owner_id"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}
