use sqlx::{Row, SqlitePool};

use crate::db::encryption::Encryptor;
use crate::db::models::*;
use crate::db::DbError;

// --- OAuth Identities ---

pub(super) async fn create_oauth_identity(
    pool: &SqlitePool,
    user_id: &str,
    provider: &str,
    provider_user_id: &str,
    provider_email: Option<&str>,
) -> Result<OAuthIdentity, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO oauth_identities (id, user_id, provider, provider_user_id, provider_email, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(provider)
    .bind(provider_user_id)
    .bind(provider_email)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            DbError::Duplicate(format!("OAuth identity for {provider} already linked"))
        }
        other => DbError::Sqlx(other),
    })?;

    Ok(OAuthIdentity {
        id,
        user_id: user_id.to_string(),
        provider: provider.to_string(),
        provider_user_id: provider_user_id.to_string(),
        provider_email: provider_email.map(String::from),
        created_at: now,
    })
}

pub(super) async fn get_oauth_identity(
    pool: &SqlitePool,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<OAuthIdentity>, DbError> {
    let identity = sqlx::query_as::<_, OAuthIdentity>(
        "SELECT * FROM oauth_identities WHERE provider = ? AND provider_user_id = ?",
    )
    .bind(provider)
    .bind(provider_user_id)
    .fetch_optional(pool)
    .await?;
    Ok(identity)
}

pub(super) async fn list_oauth_identities_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<OAuthIdentity>, DbError> {
    let identities = sqlx::query_as::<_, OAuthIdentity>(
        "SELECT * FROM oauth_identities WHERE user_id = ? ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(identities)
}

pub(super) async fn delete_oauth_identity(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM oauth_identities WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("oauth_identity {id}")));
    }
    Ok(())
}

// --- OAuth Settings ---

pub(super) async fn get_oauth_settings(
    pool: &SqlitePool,
    encryptor: &Encryptor,
) -> Result<Option<OAuthSettings>, DbError> {
    let row = sqlx::query(
        "SELECT github_client_id, github_client_secret_encrypted, github_enabled,
                google_client_id, google_client_secret_encrypted, google_enabled
         FROM oauth_settings WHERE id = 'singleton'",
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let github_secret: Option<Vec<u8>> = row.get("github_client_secret_encrypted");
            let google_secret: Option<Vec<u8>> = row.get("google_client_secret_encrypted");

            let github_client_secret = match github_secret {
                Some(enc) if !enc.is_empty() => {
                    let dec = encryptor.decrypt(&enc)?;
                    Some(String::from_utf8(dec).unwrap_or_default())
                }
                _ => None,
            };

            let google_client_secret = match google_secret {
                Some(enc) if !enc.is_empty() => {
                    let dec = encryptor.decrypt(&enc)?;
                    Some(String::from_utf8(dec).unwrap_or_default())
                }
                _ => None,
            };

            Ok(Some(OAuthSettings {
                github_client_id: row.get("github_client_id"),
                github_client_secret,
                github_enabled: row.get::<bool, _>("github_enabled"),
                google_client_id: row.get("google_client_id"),
                google_client_secret,
                google_enabled: row.get::<bool, _>("google_enabled"),
            }))
        }
        None => Ok(None),
    }
}

pub(super) async fn upsert_oauth_settings(
    pool: &SqlitePool,
    encryptor: &Encryptor,
    settings: &OAuthSettings,
) -> Result<(), DbError> {
    let now = now_iso8601();

    let github_secret_enc: Option<Vec<u8>> = match &settings.github_client_secret {
        Some(s) if !s.is_empty() => Some(encryptor.encrypt(s.as_bytes())?),
        _ => None,
    };

    let google_secret_enc: Option<Vec<u8>> = match &settings.google_client_secret {
        Some(s) if !s.is_empty() => Some(encryptor.encrypt(s.as_bytes())?),
        _ => None,
    };

    sqlx::query(
        "INSERT INTO oauth_settings (id, github_client_id, github_client_secret_encrypted, github_enabled,
                                      google_client_id, google_client_secret_encrypted, google_enabled, updated_at)
         VALUES ('singleton', ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            github_client_id = excluded.github_client_id,
            github_client_secret_encrypted = excluded.github_client_secret_encrypted,
            github_enabled = excluded.github_enabled,
            google_client_id = excluded.google_client_id,
            google_client_secret_encrypted = excluded.google_client_secret_encrypted,
            google_enabled = excluded.google_enabled,
            updated_at = excluded.updated_at",
    )
    .bind(&settings.github_client_id)
    .bind(&github_secret_enc)
    .bind(settings.github_enabled)
    .bind(&settings.google_client_id)
    .bind(&google_secret_enc)
    .bind(settings.google_enabled)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

// --- Registration Settings ---

pub(super) async fn get_registration_settings(
    pool: &SqlitePool,
) -> Result<RegistrationSettings, DbError> {
    let settings = sqlx::query_as::<_, RegistrationSettings>(
        "SELECT * FROM registration_settings WHERE id = 'singleton'",
    )
    .fetch_optional(pool)
    .await?;

    match settings {
        Some(s) => Ok(s),
        None => {
            // Return defaults if no row exists yet
            Ok(RegistrationSettings {
                id: "singleton".to_string(),
                allow_registration: false,
                allowed_domains: None,
                default_role: "viewer".to_string(),
                updated_at: now_iso8601(),
            })
        }
    }
}

pub(super) async fn upsert_registration_settings(
    pool: &SqlitePool,
    allow_registration: bool,
    allowed_domains: Option<&str>,
    default_role: &str,
) -> Result<RegistrationSettings, DbError> {
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO registration_settings (id, allow_registration, allowed_domains, default_role, updated_at)
         VALUES ('singleton', ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            allow_registration = excluded.allow_registration,
            allowed_domains = excluded.allowed_domains,
            default_role = excluded.default_role,
            updated_at = excluded.updated_at",
    )
    .bind(allow_registration)
    .bind(allowed_domains)
    .bind(default_role)
    .bind(&now)
    .execute(pool)
    .await?;

    get_registration_settings(pool).await
}
