use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_managed_db(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    db: &NewManagedDatabase,
) -> Result<ManagedDatabase, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let empty_creds = encryptor.encrypt(b"{}")?;

    sqlx::query(
        "INSERT INTO databases (id, name, db_type, credentials_encrypted, app_id, team_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&db.name)
    .bind(&db.db_type)
    .bind(&empty_creds)
    .bind(&db.app_id)
    .bind(&db.team_id)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ManagedDatabase {
        id,
        name: db.name.clone(),
        db_type: db.db_type.clone(),
        container_id: None,
        credentials: "{}".to_string(),
        backup_schedule: None,
        app_id: db.app_id.clone(),
        project_id: None,
        team_id: db.team_id.clone(),
        backup_retention_count: 7,
        created_at: now,
    })
}

pub(super) async fn update_managed_db_credentials(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    id: &str,
    credentials_json: &str,
    container_id: &str,
) -> Result<(), DbError> {
    let encrypted = encryptor.encrypt(credentials_json.as_bytes())?;
    sqlx::query("UPDATE databases SET credentials_encrypted = ?, container_id = ? WHERE id = ?")
        .bind(&encrypted)
        .bind(container_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn list_managed_dbs(
    pool: &SqlitePool,
    encryptor: &Encryptor,
) -> Result<Vec<ManagedDatabase>, DbError> {
    let rows = sqlx::query(
        "SELECT id, name, db_type, container_id, credentials_encrypted, backup_schedule, app_id, project_id, backup_retention_count, created_at
         FROM databases ORDER BY created_at DESC",
    )
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
            team_id: row.get("team_id"),
            backup_retention_count: row.get("backup_retention_count"),
            created_at: row.get("created_at"),
        });
    }
    Ok(dbs)
}

pub(super) async fn list_managed_dbs_by_project(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    project_id: &str,
) -> Result<Vec<ManagedDatabase>, DbError> {
    let rows = sqlx::query(
        "SELECT id, name, db_type, container_id, credentials_encrypted, backup_schedule, app_id, project_id, backup_retention_count, created_at
         FROM databases WHERE project_id = ? ORDER BY created_at DESC",
    )
    .bind(project_id)
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
            team_id: row.get("team_id"),
            backup_retention_count: row.get("backup_retention_count"),
            created_at: row.get("created_at"),
        });
    }
    Ok(dbs)
}

pub(super) async fn delete_managed_db(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM databases WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
