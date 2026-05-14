use sqlx::{Row, SqlitePool};

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn list_ssh_keys(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<SshKey>, DbError> {
    let rows = sqlx::query(
        "SELECT id, user_id, name, public_key, private_key_encrypted, fingerprint, key_type, last_used_at, created_at
         FROM ssh_keys WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
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

pub(super) async fn create_ssh_key(pool: &SqlitePool, key: &NewSshKey) -> Result<SshKey, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO ssh_keys (id, user_id, name, public_key, private_key_encrypted, fingerprint, key_type, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&key.user_id)
    .bind(&key.name)
    .bind(&key.public_key)
    .bind(&key.private_key_encrypted)
    .bind(&key.fingerprint)
    .bind(&key.key_type)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(SshKey {
        id,
        user_id: key.user_id.clone(),
        name: key.name.clone(),
        public_key: key.public_key.clone(),
        private_key_encrypted: key.private_key_encrypted.clone(),
        fingerprint: key.fingerprint.clone(),
        key_type: key.key_type.clone(),
        last_used_at: None,
        created_at: now,
    })
}

pub(super) async fn delete_ssh_key(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM ssh_keys WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn get_ssh_key(pool: &SqlitePool, id: &str) -> Result<Option<SshKey>, DbError> {
    let row = sqlx::query(
        "SELECT id, user_id, name, public_key, private_key_encrypted, fingerprint, key_type, last_used_at, created_at
         FROM ssh_keys WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| SshKey {
        id: r.get("id"),
        user_id: r.get("user_id"),
        name: r.get("name"),
        public_key: r.get("public_key"),
        private_key_encrypted: r.get("private_key_encrypted"),
        fingerprint: r.get("fingerprint"),
        key_type: r.get("key_type"),
        last_used_at: r.get("last_used_at"),
        created_at: r.get("created_at"),
    }))
}
