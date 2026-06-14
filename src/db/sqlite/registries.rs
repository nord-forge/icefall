use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn list_registries(
    pool: &SqlitePool,
    encryptor: &Encryptor,
) -> Result<Vec<Registry>, DbError> {
    let rows = sqlx::query(
        "SELECT id, name, url, username_encrypted, password_encrypted, registry_type, created_at, updated_at
         FROM registries ORDER BY created_at DESC",
    )
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

pub(super) async fn create_registry(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    reg: &NewRegistry,
) -> Result<Registry, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let username_enc = encryptor.encrypt(reg.username.as_bytes())?;
    let password_enc = encryptor.encrypt(reg.password.as_bytes())?;

    sqlx::query(
        "INSERT INTO registries (id, name, url, username_encrypted, password_encrypted, registry_type, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&reg.name)
    .bind(&reg.url)
    .bind(&username_enc)
    .bind(&password_enc)
    .bind(&reg.registry_type)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(Registry {
        id,
        name: reg.name.clone(),
        url: reg.url.clone(),
        username: reg.username.clone(),
        password: reg.password.clone(),
        registry_type: reg.registry_type.clone(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(super) async fn delete_registry(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM registries WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
