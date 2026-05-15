use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

// --- Sessions ---

pub(super) async fn create_session(
    pool: &SqlitePool,
    user_id: &str,
    expires_at: &str,
) -> Result<Session, DbError> {
    let id = new_id();
    let now = now_iso8601();
    sqlx::query("INSERT INTO sessions (id, user_id, expires_at, created_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(user_id)
        .bind(expires_at)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(Session {
        id,
        user_id: user_id.to_string(),
        active_team_id: None,
        expires_at: expires_at.to_string(),
        created_at: now,
    })
}

pub(super) async fn get_session(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Option<Session>, DbError> {
    Ok(
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?,
    )
}

pub(super) async fn delete_session(pool: &SqlitePool, session_id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn delete_user_sessions(pool: &SqlitePool, user_id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn list_user_sessions(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<Session>, DbError> {
    Ok(sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub(super) async fn delete_user_sessions_except(
    pool: &SqlitePool,
    user_id: &str,
    keep_session_id: &str,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ? AND id != ?")
        .bind(user_id)
        .bind(keep_session_id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- API Tokens ---

pub(super) async fn create_api_token(
    pool: &SqlitePool,
    user_id: &str,
    name: &str,
    token_hash: &str,
    expires_at: Option<&str>,
    team_id: Option<&str>,
) -> Result<ApiToken, DbError> {
    let id = new_id();
    let now = now_iso8601();
    sqlx::query("INSERT INTO api_tokens (id, user_id, name, token_hash, expires_at, team_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(user_id).bind(name).bind(token_hash).bind(expires_at).bind(team_id).bind(&now)
        .execute(pool).await?;
    Ok(ApiToken {
        id,
        user_id: user_id.to_string(),
        name: name.to_string(),
        token_hash: token_hash.to_string(),
        team_id: team_id.map(String::from),
        last_used_at: None,
        expires_at: expires_at.map(String::from),
        created_at: now,
    })
}

pub(super) async fn get_api_token_by_hash(
    pool: &SqlitePool,
    token_hash: &str,
) -> Result<Option<ApiToken>, DbError> {
    Ok(
        sqlx::query_as::<_, ApiToken>("SELECT * FROM api_tokens WHERE token_hash = ?")
            .bind(token_hash)
            .fetch_optional(pool)
            .await?,
    )
}

pub(super) async fn list_api_tokens(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<ApiToken>, DbError> {
    Ok(sqlx::query_as::<_, ApiToken>(
        "SELECT * FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub(super) async fn delete_api_token(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM api_tokens WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_token_last_used(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- Invitations ---

pub(super) async fn create_invitation(
    pool: &SqlitePool,
    email: &str,
    role: &str,
    token: &str,
    expires_at: &str,
) -> Result<Invitation, DbError> {
    let id = new_id();
    let now = now_iso8601();
    sqlx::query("INSERT INTO invitations (id, email, role, token, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(email).bind(role).bind(token).bind(expires_at).bind(&now)
        .execute(pool).await?;
    Ok(Invitation {
        id,
        email: email.to_string(),
        role: role.to_string(),
        token: token.to_string(),
        expires_at: expires_at.to_string(),
        created_at: now,
    })
}

pub(super) async fn get_invitation_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<Invitation>, DbError> {
    Ok(
        sqlx::query_as::<_, Invitation>("SELECT * FROM invitations WHERE token = ?")
            .bind(token)
            .fetch_optional(pool)
            .await?,
    )
}

pub(super) async fn delete_invitation(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM invitations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
