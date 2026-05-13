use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

// --- Environments ---

pub(super) async fn create_environment(
    pool: &SqlitePool,
    env: &NewEnvironment,
) -> Result<Environment, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO environments (id, app_id, name, env_type, branch, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&env.app_id)
    .bind(&env.name)
    .bind(&env.env_type)
    .bind(&env.branch)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(Environment {
        id,
        app_id: env.app_id.clone(),
        name: env.name.clone(),
        env_type: env.env_type.clone(),
        branch: env.branch.clone(),
        created_at: now,
    })
}

pub(super) async fn list_environments(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Vec<Environment>, DbError> {
    let envs = sqlx::query_as::<_, Environment>(
        "SELECT * FROM environments WHERE app_id = ? ORDER BY created_at",
    )
    .bind(app_id)
    .fetch_all(pool)
    .await?;
    Ok(envs)
}

pub(super) async fn delete_environment(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM environments WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn get_environment_by_branch(
    pool: &SqlitePool,
    app_id: &str,
    branch: &str,
) -> Result<Option<Environment>, DbError> {
    let env = sqlx::query_as::<_, Environment>(
        "SELECT * FROM environments WHERE app_id = ? AND branch = ?",
    )
    .bind(app_id)
    .bind(branch)
    .fetch_optional(pool)
    .await?;
    Ok(env)
}

// --- Env Vars (encrypted) ---

pub(super) async fn set_env_var(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    env_var: &NewEnvVar,
) -> Result<EnvVar, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let encrypted_value = encryptor.encrypt(env_var.value.as_bytes())?;

    sqlx::query(
        "INSERT INTO env_vars (id, environment_id, key, value_encrypted, scope, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(environment_id, key, scope) DO UPDATE SET value_encrypted = excluded.value_encrypted",
    )
    .bind(&id)
    .bind(&env_var.environment_id)
    .bind(&env_var.key)
    .bind(&encrypted_value)
    .bind(&env_var.scope)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(EnvVar {
        id,
        environment_id: env_var.environment_id.clone(),
        key: env_var.key.clone(),
        value: env_var.value.clone(),
        scope: env_var.scope.clone(),
        created_at: now,
    })
}

pub(super) async fn get_env_vars(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    environment_id: &str,
) -> Result<Vec<EnvVar>, DbError> {
    let rows = sqlx::query(
        "SELECT id, environment_id, key, value_encrypted, scope, created_at
         FROM env_vars WHERE environment_id = ? ORDER BY key",
    )
    .bind(environment_id)
    .fetch_all(pool)
    .await?;

    let mut env_vars = Vec::with_capacity(rows.len());
    for row in rows {
        let encrypted: Vec<u8> = row.get("value_encrypted");
        let decrypted = encryptor.decrypt(&encrypted)?;
        let value = String::from_utf8(decrypted).unwrap_or_default();

        env_vars.push(EnvVar {
            id: row.get("id"),
            environment_id: row.get("environment_id"),
            key: row.get("key"),
            value,
            scope: row.get("scope"),
            created_at: row.get("created_at"),
        });
    }
    Ok(env_vars)
}

pub(super) async fn delete_env_var(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM env_vars WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn delete_env_vars_by_environment(
    pool: &SqlitePool,
    environment_id: &str,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM env_vars WHERE environment_id = ?")
        .bind(environment_id)
        .execute(pool)
        .await?;
    Ok(())
}
