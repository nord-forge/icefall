use sqlx::{Row, SqlitePool};

use crate::db::models::*;
use crate::db::DbError;

// --- Users ---

pub(super) async fn create_user(pool: &SqlitePool, user: &NewUser) -> Result<User, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.role)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            DbError::Duplicate(format!("user '{}' already exists", user.email))
        }
        other => DbError::Sqlx(other),
    })?;

    Ok(User {
        id,
        email: user.email.clone(),
        password_hash: user.password_hash.clone(),
        role: user.role.clone(),
        totp_secret: None,
        totp_enabled: false,
        totp_backup_codes: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(super) async fn get_user_by_email(
    pool: &SqlitePool,
    email: &str,
) -> Result<Option<User>, DbError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub(super) async fn get_user_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>, DbError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub(super) async fn update_user_totp_secret(
    pool: &SqlitePool,
    user_id: &str,
    secret: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET totp_secret = ?, updated_at = ? WHERE id = ?")
        .bind(secret)
        .bind(now_iso8601())
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn enable_user_totp(
    pool: &SqlitePool,
    user_id: &str,
    backup_codes: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE users SET totp_enabled = 1, totp_backup_codes = ?, updated_at = ? WHERE id = ?",
    )
    .bind(backup_codes)
    .bind(now_iso8601())
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn disable_user_totp(pool: &SqlitePool, user_id: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET totp_enabled = 0, totp_secret = NULL, totp_backup_codes = NULL, updated_at = ? WHERE id = ?")
        .bind(now_iso8601())
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_user_backup_codes(
    pool: &SqlitePool,
    user_id: &str,
    backup_codes: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET totp_backup_codes = ?, updated_at = ? WHERE id = ?")
        .bind(backup_codes)
        .bind(now_iso8601())
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn list_users(pool: &SqlitePool) -> Result<Vec<User>, DbError> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at")
        .fetch_all(pool)
        .await?;
    Ok(users)
}

// --- User Profile Updates ---

pub(super) async fn update_user_password(
    pool: &SqlitePool,
    user_id: &str,
    password_hash: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(password_hash)
        .bind(now_iso8601())
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_user_email(
    pool: &SqlitePool,
    user_id: &str,
    email: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
        .bind(email)
        .bind(now_iso8601())
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
                DbError::Duplicate(format!("email '{}' is already in use", email))
            }
            other => DbError::Sqlx(other),
        })?;
    Ok(())
}

// --- User Deletion ---

pub(super) async fn delete_user(pool: &SqlitePool, user_id: &str) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("user {user_id}")));
    }
    Ok(())
}

pub(super) async fn count_admin_users(pool: &SqlitePool) -> Result<i64, DbError> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE role = 'admin'")
        .fetch_one(pool)
        .await?;
    Ok(row.get::<i64, _>("count"))
}

// --- User Preferences ---

pub(super) async fn get_user_preferences(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<serde_json::Value, DbError> {
    let row = sqlx::query("SELECT preferences FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some(row) => {
            let raw: String = row.get("preferences");
            let value: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));
            Ok(value)
        }
        None => Err(DbError::NotFound(format!("user {user_id}"))),
    }
}

pub(super) async fn update_user_preferences(
    pool: &SqlitePool,
    user_id: &str,
    preferences: &serde_json::Value,
) -> Result<(), DbError> {
    let json_str = serde_json::to_string(preferences)
        .map_err(|e| DbError::Sqlx(sqlx::Error::Protocol(e.to_string())))?;
    let result = sqlx::query("UPDATE users SET preferences = ?, updated_at = ? WHERE id = ?")
        .bind(&json_str)
        .bind(now_iso8601())
        .bind(user_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("user {user_id}")));
    }
    Ok(())
}

// --- Admin 2FA reset ---

pub(super) async fn admin_reset_user_2fa(pool: &SqlitePool, user_id: &str) -> Result<(), DbError> {
    let now = now_iso8601();
    let result = sqlx::query(
        "UPDATE users SET totp_enabled = 0, totp_secret = NULL, totp_backup_codes = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("user {user_id}")));
    }
    Ok(())
}
