use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct UpdateSslRequest {
    ssl_enabled: bool,
    ssl_mode: Option<String>,
}

pub async fn update_database_ssl(
    State(state): State<AppState>,
    Path(db_id): Path<String>,
    Json(body): Json<UpdateSslRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .db
        .update_database_ssl(&db_id, body.ssl_enabled, body.ssl_mode.as_deref())
        .await?;

    Ok(Json(serde_json::json!({ "message": "updated" })))
}

pub async fn get_database_certificate(
    State(state): State<AppState>,
    Path(db_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let managed_dbs = state.db.list_managed_dbs().await?;
    let db_record = managed_dbs
        .iter()
        .find(|d| d.id == db_id)
        .ok_or_else(|| ApiError::NotFound(format!("database {db_id}")))?;

    Ok(Json(serde_json::json!({
        "data": {
            "ssl_enabled": db_record.ssl_enabled,
            "ssl_mode": db_record.ssl_mode,
            "ssl_ca_cert": db_record.ssl_ca_cert,
            "ssl_expires_at": db_record.ssl_expires_at,
        }
    })))
}

pub async fn regenerate_certificate(
    State(state): State<AppState>,
    Path(db_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (ca_cert, cert, key, expires_at) = generate_self_signed_certs(&db_id)?;

    state
        .db
        .store_database_certs(&db_id, &ca_cert, &cert, &key, &expires_at)
        .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "ca_cert": ca_cert,
            "expires_at": expires_at,
            "message": "Certificate regenerated",
        }
    })))
}

fn generate_self_signed_certs(db_id: &str) -> Result<(String, String, String, String), ApiError> {
    use rcgen::{generate_simple_self_signed, CertifiedKey};

    let subject_alt_names = vec![format!("{db_id}.icefall.internal"), "localhost".to_string()];

    let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)
        .map_err(|e| ApiError::BadRequest(format!("Certificate generation failed: {e}")))?;

    let cert_pem = cert.pem();
    let key_pem = signing_key.serialize_pem();

    let expires_at = (chrono::Utc::now() + chrono::Duration::days(365)).to_rfc3339();

    Ok((cert_pem.clone(), cert_pem, key_pem, expires_at))
}
