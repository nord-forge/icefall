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

    Ok(rows
        .into_iter()
        .map(|r| GitHubInstallation {
            id: r.get("id"),
            installation_id: r.get("installation_id"),
            account_login: r.get("account_login"),
            account_type: r.get("account_type"),
            token_expires_at: r.get("token_expires_at"),
            github_app_id: r.get("github_app_id"),
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
