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

/// Insert a personal team for `user_id` plus an `owner` membership inside the
/// given transaction, so `team_id` is always resolvable ("always-a-team" model).
async fn insert_personal_team(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    user_id: &str,
    now: &str,
) -> Result<Team, DbError> {
    let team_id = new_id();
    // Slug must be unique; derive it from the user id so it never collides.
    let slug = format!("personal-{user_id}");

    sqlx::query(
        "INSERT INTO teams (id, name, slug, owner_id, created_at, updated_at)
         VALUES (?, 'Personal', ?, ?, ?, ?)",
    )
    .bind(&team_id)
    .bind(&slug)
    .bind(user_id)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO team_memberships (id, team_id, user_id, role, accepted_at, created_at)
         VALUES (?, ?, ?, 'owner', ?, ?)",
    )
    .bind(new_id())
    .bind(&team_id)
    .bind(user_id)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;

    Ok(Team {
        id: team_id,
        name: "Personal".to_string(),
        slug,
        owner_id: user_id.to_string(),
        settings: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    })
}

/// Atomically create the very first admin account with a personal team. The
/// `INSERT ... WHERE NOT EXISTS` guard gives racing losers `DbError::Duplicate`.
pub(super) async fn create_first_admin(pool: &SqlitePool, user: &NewUser) -> Result<User, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        "INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
         SELECT ?, ?, ?, 'admin', ?, ?
         WHERE NOT EXISTS (SELECT 1 FROM users)",
    )
    .bind(&id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            DbError::Duplicate("an admin account already exists".to_string())
        }
        other => DbError::Sqlx(other),
    })?;

    if result.rows_affected() == 0 {
        return Err(DbError::Duplicate(
            "an admin account already exists".to_string(),
        ));
    }

    insert_personal_team(&mut tx, &id, &now).await?;
    tx.commit().await?;

    Ok(User {
        id,
        email: user.email.clone(),
        password_hash: user.password_hash.clone(),
        role: "admin".to_string(),
        totp_secret: None,
        totp_enabled: false,
        totp_backup_codes: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Create a user together with their personal team, atomically — the standard
/// user-creation path (OAuth sign-up, invitation accept).
pub(super) async fn create_user_with_personal_team(
    pool: &SqlitePool,
    user: &NewUser,
) -> Result<(User, Team), DbError> {
    let id = new_id();
    let now = now_iso8601();
    let mut tx = pool.begin().await?;

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
    .execute(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            DbError::Duplicate(format!("user '{}' already exists", user.email))
        }
        other => DbError::Sqlx(other),
    })?;

    let team = insert_personal_team(&mut tx, &id, &now).await?;
    tx.commit().await?;

    let user = User {
        id,
        email: user.email.clone(),
        password_hash: user.password_hash.clone(),
        role: user.role.clone(),
        totp_secret: None,
        totp_enabled: false,
        totp_backup_codes: None,
        created_at: now.clone(),
        updated_at: now,
    };
    Ok((user, team))
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        sqlx::migrate!("src/db/migrations")
            .run(&pool)
            .await
            .expect("migrate");
        pool
    }

    fn new_admin(email: &str) -> NewUser {
        NewUser {
            email: email.to_string(),
            password_hash: "$argon2id$test".to_string(),
            role: "admin".to_string(),
        }
    }

    #[tokio::test]
    async fn create_first_admin_succeeds_on_empty_db() {
        let pool = pool().await;
        let user = create_first_admin(&pool, &new_admin("admin@example.com"))
            .await
            .expect("first admin created");
        assert_eq!(user.role, "admin");
        assert_eq!(user.email, "admin@example.com");
    }

    #[tokio::test]
    async fn create_first_admin_rejected_when_a_user_exists() {
        let pool = pool().await;
        create_first_admin(&pool, &new_admin("first@example.com"))
            .await
            .expect("first admin created");

        // audit H8: a second create_first_admin must not produce another admin.
        let err = create_first_admin(&pool, &new_admin("second@example.com"))
            .await
            .expect_err("second admin must be rejected");
        assert!(matches!(err, DbError::Duplicate(_)));

        let users = list_users(&pool).await.expect("list users");
        assert_eq!(users.len(), 1);
    }

    #[tokio::test]
    async fn create_first_admin_is_atomic_under_concurrency() {
        let pool = pool().await;

        // Fire many concurrent setup requests against the same fresh DB; the
        // INSERT ... WHERE NOT EXISTS guard must let exactly one through.
        let mut handles = Vec::new();
        for i in 0..16 {
            let pool = pool.clone();
            handles.push(tokio::spawn(async move {
                create_first_admin(&pool, &new_admin(&format!("admin{i}@example.com"))).await
            }));
        }

        let mut successes = 0;
        for h in handles {
            if h.await.expect("join task").is_ok() {
                successes += 1;
            }
        }
        assert_eq!(successes, 1, "exactly one admin should be created");
        assert_eq!(list_users(&pool).await.unwrap().len(), 1);
    }
}
