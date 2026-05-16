use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

// --- List by team ---

pub(super) async fn list_apps_by_team(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<App>, DbError> {
    Ok(
        sqlx::query_as::<_, App>("SELECT * FROM apps WHERE team_id = ? ORDER BY created_at DESC")
            .bind(team_id)
            .fetch_all(pool)
            .await?,
    )
}

pub(super) async fn list_projects_by_team(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<Project>, DbError> {
    Ok(
        sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE team_id = ? ORDER BY name ASC")
            .bind(team_id)
            .fetch_all(pool)
            .await?,
    )
}

pub(super) async fn list_managed_dbs_by_team(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    team_id: &str,
) -> Result<Vec<ManagedDatabase>, DbError> {
    let rows = sqlx::query(
        "SELECT id, name, db_type, container_id, credentials_encrypted, backup_schedule, app_id, project_id, backup_retention_count, created_at
         FROM databases WHERE team_id = ? ORDER BY created_at DESC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?;

    let mut dbs = Vec::with_capacity(rows.len());
    for row in rows {
        let encrypted: Vec<u8> = row.get("credentials_encrypted");
        let decrypted = encryptor.decrypt(&encrypted)?;
        let credentials = String::from_utf8(decrypted).unwrap_or_default();

        dbs.push(ManagedDatabase {
            id: row.get("id"),
            name: row.get("name"),
            db_type: row.get("db_type"),
            container_id: row.get("container_id"),
            credentials,
            backup_schedule: row.get("backup_schedule"),
            app_id: row.get("app_id"),
            project_id: row.get("project_id"),
            backup_retention_count: row.get("backup_retention_count"),
            created_at: row.get("created_at"),
        });
    }
    Ok(dbs)
}

pub(super) async fn list_ssh_keys_by_team(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<SshKey>, DbError> {
    let rows = sqlx::query(
        "SELECT id, user_id, name, public_key, private_key_encrypted, fingerprint, key_type, last_used_at, created_at
         FROM ssh_keys WHERE team_id = ? ORDER BY created_at DESC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SshKey {
            id: r.get("id"),
            user_id: r.get("user_id"),
            name: r.get("name"),
            public_key: r.get("public_key"),
            private_key_encrypted: r.get("private_key_encrypted"),
            fingerprint: r.get("fingerprint"),
            key_type: r.get("key_type"),
            last_used_at: r.get("last_used_at"),
            created_at: r.get("created_at"),
        })
        .collect())
}

pub(super) async fn list_registries_by_team(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    team_id: &str,
) -> Result<Vec<Registry>, DbError> {
    let rows = sqlx::query(
        "SELECT id, name, url, username_encrypted, password_encrypted, registry_type, created_at, updated_at
         FROM registries WHERE team_id = ? ORDER BY created_at DESC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?;

    let mut regs = Vec::with_capacity(rows.len());
    for r in rows {
        let username_enc: Vec<u8> = r.get("username_encrypted");
        let password_enc: Vec<u8> = r.get("password_encrypted");
        let username = String::from_utf8(encryptor.decrypt(&username_enc)?).unwrap_or_default();
        let password = String::from_utf8(encryptor.decrypt(&password_enc)?).unwrap_or_default();

        regs.push(Registry {
            id: r.get("id"),
            name: r.get("name"),
            url: r.get("url"),
            username,
            password,
            registry_type: r.get("registry_type"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        });
    }
    Ok(regs)
}

pub(super) async fn list_notification_channels_by_team(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    team_id: &str,
) -> Result<Vec<Notification>, DbError> {
    let rows = sqlx::query(
        "SELECT id, channel_type, config_encrypted, created_at FROM notifications WHERE team_id = ? ORDER BY created_at",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?;

    let mut channels = Vec::with_capacity(rows.len());
    for row in rows {
        let encrypted: Vec<u8> = row.get("config_encrypted");
        let decrypted = encryptor.decrypt(&encrypted)?;
        let config = String::from_utf8(decrypted).unwrap_or_default();

        channels.push(Notification {
            id: row.get("id"),
            channel_type: row.get("channel_type"),
            config,
            created_at: row.get("created_at"),
        });
    }
    Ok(channels)
}

pub(super) async fn list_api_tokens_by_team(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<ApiToken>, DbError> {
    Ok(sqlx::query_as::<_, ApiToken>(
        "SELECT * FROM api_tokens WHERE team_id = ? ORDER BY created_at DESC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?)
}

// --- Set team on resources ---

pub(super) async fn set_app_team(
    pool: &SqlitePool,
    app_id: &str,
    team_id: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE apps SET team_id = ? WHERE id = ?")
        .bind(team_id)
        .bind(app_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn set_project_team(
    pool: &SqlitePool,
    project_id: &str,
    team_id: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE projects SET team_id = ? WHERE id = ?")
        .bind(team_id)
        .bind(project_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn set_database_team(
    pool: &SqlitePool,
    db_id: &str,
    team_id: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE databases SET team_id = ? WHERE id = ?")
        .bind(team_id)
        .bind(db_id)
        .execute(pool)
        .await?;
    Ok(())
}
