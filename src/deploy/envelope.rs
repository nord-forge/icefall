use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::PublicKey;

use crate::deploy::DeployError;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretEnvelope {
    pub encrypted_key: String,
    pub ephemeral_public: String,
    pub nonce: String,
    pub ciphertext: String,
    pub key_id: String,
}

pub fn encrypt_env_vars(
    env_vars: &[String],
    server_public_key_b64: &str,
) -> Result<SecretEnvelope, DeployError> {
    let server_pub_bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(server_public_key_b64)
        .map_err(|e| DeployError::EnvelopeEncrypt(format!("invalid server public key: {e}")))?
        .try_into()
        .map_err(|_| DeployError::EnvelopeEncrypt("public key must be 32 bytes".into()))?;

    let server_public = PublicKey::from(server_pub_bytes);

    let mut ephemeral_bytes = [0u8; 32];
    rand::RngExt::fill(&mut rand::rng(), &mut ephemeral_bytes);
    let ephemeral_secret = x25519_dalek::StaticSecret::from(ephemeral_bytes);
    let ephemeral_public = x25519_dalek::PublicKey::from(&ephemeral_secret);

    let shared_secret = ephemeral_secret.diffie_hellman(&server_public);

    // Derive AES-256 key from the shared secret via SHA-256
    let aes_key_bytes = Sha256::digest(shared_secret.as_bytes());

    let cipher = Aes256Gcm::new_from_slice(&aes_key_bytes)
        .map_err(|e| DeployError::EnvelopeEncrypt(format!("AES key init failed: {e}")))?;

    let mut nonce_bytes = [0u8; 12];
    rand::RngExt::fill(&mut rand::rng(), &mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = serde_json::to_vec(env_vars)
        .map_err(|e| DeployError::EnvelopeEncrypt(format!("failed to serialize env vars: {e}")))?;

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| DeployError::EnvelopeEncrypt(format!("AES encryption failed: {e}")))?;

    let key_fingerprint = {
        let hash = Sha256::digest(server_pub_bytes);
        format!("sha256:{}", hex::encode(&hash[..8]))
    };

    Ok(SecretEnvelope {
        encrypted_key: String::new(), // shared secret derived via DH, no encrypted key needed
        ephemeral_public: URL_SAFE_NO_PAD.encode(ephemeral_public.as_bytes()),
        nonce: URL_SAFE_NO_PAD.encode(nonce_bytes),
        ciphertext: URL_SAFE_NO_PAD.encode(&ciphertext),
        key_id: key_fingerprint,
    })
}

pub fn decrypt_env_vars(
    envelope: &SecretEnvelope,
    private_key_b64: &str,
) -> Result<Vec<String>, String> {
    let private_bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(private_key_b64)
        .map_err(|e| format!("invalid private key: {e}"))?
        .try_into()
        .map_err(|_| "private key must be 32 bytes".to_string())?;

    let ephemeral_pub_bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(&envelope.ephemeral_public)
        .map_err(|e| format!("invalid ephemeral public key: {e}"))?
        .try_into()
        .map_err(|_| "ephemeral public key must be 32 bytes".to_string())?;

    let secret = x25519_dalek::StaticSecret::from(private_bytes);
    let ephemeral_public = PublicKey::from(ephemeral_pub_bytes);

    let shared_secret = secret.diffie_hellman(&ephemeral_public);
    let aes_key_bytes = Sha256::digest(shared_secret.as_bytes());

    let cipher = Aes256Gcm::new_from_slice(&aes_key_bytes)
        .map_err(|e| format!("AES key init failed: {e}"))?;

    let nonce_bytes = URL_SAFE_NO_PAD
        .decode(&envelope.nonce)
        .map_err(|e| format!("invalid nonce: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = URL_SAFE_NO_PAD
        .decode(&envelope.ciphertext)
        .map_err(|e| format!("invalid ciphertext: {e}"))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "unable to decrypt environment variables".to_string())?;

    let env_vars: Vec<String> =
        serde_json::from_slice(&plaintext).map_err(|e| format!("invalid env var format: {e}"))?;

    Ok(env_vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypt_decrypt() {
        let secret = {
            let mut bytes = [0u8; 32];
            rand::RngExt::fill(&mut rand::rng(), &mut bytes);
            x25519_dalek::StaticSecret::from(bytes)
        };
        let public = x25519_dalek::PublicKey::from(&secret);

        let pub_b64 = URL_SAFE_NO_PAD.encode(public.as_bytes());
        let priv_b64 = URL_SAFE_NO_PAD.encode(secret.to_bytes());

        let env_vars = vec![
            "DATABASE_URL=postgres://user:pass@host/db".to_string(),
            "API_KEY=secret123".to_string(),
        ];

        let envelope = encrypt_env_vars(&env_vars, &pub_b64).unwrap();
        let decrypted = decrypt_env_vars(&envelope, &priv_b64).unwrap();

        assert_eq!(decrypted, env_vars);
    }

    #[test]
    fn wrong_key_fails() {
        let secret = {
            let mut bytes = [0u8; 32];
            rand::RngExt::fill(&mut rand::rng(), &mut bytes);
            x25519_dalek::StaticSecret::from(bytes)
        };
        let public = x25519_dalek::PublicKey::from(&secret);

        let wrong_secret = {
            let mut bytes = [0u8; 32];
            rand::RngExt::fill(&mut rand::rng(), &mut bytes);
            x25519_dalek::StaticSecret::from(bytes)
        };

        let pub_b64 = URL_SAFE_NO_PAD.encode(public.as_bytes());
        let wrong_priv_b64 = URL_SAFE_NO_PAD.encode(wrong_secret.to_bytes());

        let env_vars = vec!["SECRET=value".to_string()];
        let envelope = encrypt_env_vars(&env_vars, &pub_b64).unwrap();

        let result = decrypt_env_vars(&envelope, &wrong_priv_b64);
        assert!(result.is_err());
    }
}
