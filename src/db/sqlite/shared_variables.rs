use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn list_shared_variables(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    scope: &str,
    scope_id: &str,
) -> Result<Vec<SharedVariable>, DbError> {
    let rows = sqlx::query(
        "SELECT id, scope, scope_id, key, value_encrypted, is_sensitive, created_at, updated_at
         FROM shared_variables WHERE scope = ? AND scope_id = ? ORDER BY key ASC",
    )
    .bind(scope)
    .bind(scope_id)
    .fetch_all(pool)
    .await?;

    let mut vars = Vec::with_capacity(rows.len());
    for row in rows {
        let encrypted: Vec<u8> = row.get("value_encrypted");
        let decrypted = encryptor.decrypt(&encrypted)?;
        let value = String::from_utf8(decrypted).unwrap_or_default();

        vars.push(SharedVariable {
            id: row.get("id"),
            scope: row.get("scope"),
            scope_id: row.get("scope_id"),
            key: row.get("key"),
            value,
            is_sensitive: row.get("is_sensitive"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }
    Ok(vars)
}

pub(super) async fn set_shared_variable(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    var: &NewSharedVariable,
) -> Result<SharedVariable, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let encrypted_value = encryptor.encrypt(var.value.as_bytes())?;

    sqlx::query(
        "INSERT INTO shared_variables (id, scope, scope_id, key, value_encrypted, is_sensitive, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(scope, scope_id, key) DO UPDATE SET
             value_encrypted = excluded.value_encrypted,
             is_sensitive = excluded.is_sensitive,
             updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(&var.scope)
    .bind(&var.scope_id)
    .bind(&var.key)
    .bind(&encrypted_value)
    .bind(var.is_sensitive)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(SharedVariable {
        id,
        scope: var.scope.clone(),
        scope_id: var.scope_id.clone(),
        key: var.key.clone(),
        value: var.value.clone(),
        is_sensitive: var.is_sensitive,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(super) async fn delete_shared_variable(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM shared_variables WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn get_shared_variables_for_app(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    app_id: &str,
) -> Result<Vec<SharedVariable>, DbError> {
    // First, fetch the app to get its project_id and server_id
    let app = sqlx::query("SELECT project_id, server_id FROM apps WHERE id = ?")
        .bind(app_id)
        .fetch_optional(pool)
        .await?;

    let Some(app_row) = app else {
        return Ok(Vec::new());
    };

    let project_id: Option<String> = app_row.get("project_id");
    let server_id: Option<String> = app_row.get("server_id");

    // Return empty if app has neither project nor server
    let (pid, sid) = match (project_id, server_id) {
        (None, None) => return Ok(Vec::new()),
        (p, s) => (p.unwrap_or_default(), s.unwrap_or_default()),
    };

    let rows = sqlx::query(
        "SELECT id, scope, scope_id, key, value_encrypted, is_sensitive, created_at, updated_at
         FROM shared_variables
         WHERE (scope = 'project' AND scope_id = ?)
            OR (scope = 'server' AND scope_id = ?)
         ORDER BY key ASC",
    )
    .bind(&pid)
    .bind(&sid)
    .fetch_all(pool)
    .await?;

    let mut vars = Vec::with_capacity(rows.len());
    for row in rows {
        let encrypted: Vec<u8> = row.get("value_encrypted");
        let decrypted = encryptor.decrypt(&encrypted)?;
        let value = String::from_utf8(decrypted).unwrap_or_default();

        vars.push(SharedVariable {
            id: row.get("id"),
            scope: row.get("scope"),
            scope_id: row.get("scope_id"),
            key: row.get("key"),
            value,
            is_sensitive: row.get("is_sensitive"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }
    Ok(vars)
}
