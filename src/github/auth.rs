use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;

#[derive(Serialize)]
struct Claims {
    iss: String,
    iat: i64,
    exp: i64,
}

/// Generate a JWT for authenticating as a GitHub App: RSA-signed, valid for
/// 10 minutes, with `iat` backdated 60s to account for clock drift.
pub fn generate_jwt(app_id: i64, private_key_pem: &str) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        iss: app_id.to_string(),
        iat: now - 60,
        exp: now + (10 * 60),
    };
    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| format!("Invalid private key: {e}"))?;
    let header = Header::new(Algorithm::RS256);
    encode(&header, &claims, &key).map_err(|e| format!("JWT encode failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_jwt_fails_with_invalid_key() {
        let result = generate_jwt(12345, "not-a-valid-pem");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid private key"));
    }
}
