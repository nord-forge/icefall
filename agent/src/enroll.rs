use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct EnrollResponse {
    data: EnrollData,
}

#[derive(Deserialize)]
struct EnrollData {
    worker_token: String,
    server_id: String,
}

pub async fn run_enrollment(url: &str, token: &str, data_dir: &str) -> Result<(), String> {
    let url = url.trim_end_matches('/');

    info!(
        "Enrolling with {}... (token: {}...)",
        url,
        &token[..8.min(token.len())]
    );

    // Generate X25519 keypair
    use rand::RngExt;
    let secret_bytes: [u8; 32] = rand::rng().random();
    let secret_key = x25519_dalek::StaticSecret::from(secret_bytes);
    let public_key = x25519_dalek::PublicKey::from(&secret_key);
    let public_key_b64 = URL_SAFE_NO_PAD.encode(public_key.as_bytes());

    // Call enrollment endpoint
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{url}/api/v1/agent/enroll"))
        .json(&serde_json::json!({
            "enrollment_token": token,
            "public_key": public_key_b64,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to control plane: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Enrollment failed (HTTP {status}): {body}"));
    }

    let enroll_resp: EnrollResponse = response
        .json()
        .await
        .map_err(|e| format!("Invalid enrollment response: {e}"))?;

    info!("Enrolled as server {}", enroll_resp.data.server_id);

    // Store private key
    let keys_dir = Path::new(data_dir).join("keys");
    fs::create_dir_all(&keys_dir).map_err(|e| format!("Failed to create keys directory: {e}"))?;

    let private_key_path = keys_dir.join("private.key");
    let private_key_b64 = URL_SAFE_NO_PAD.encode(secret_key.to_bytes());
    fs::write(&private_key_path, &private_key_b64)
        .map_err(|e| format!("Failed to write private key: {e}"))?;
    fs::set_permissions(&private_key_path, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("Failed to set private key permissions: {e}"))?;

    // Write config file
    let config_dir = Path::new(data_dir);
    fs::create_dir_all(config_dir)
        .map_err(|e| format!("Failed to create config directory: {e}"))?;

    let config_path = config_dir.join("config.toml");
    let config_content = format!(
        r#"control_plane_url = "{url}"
token = "{}"
server_id = "{}"
log_level = "info"
"#,
        enroll_resp.data.worker_token, enroll_resp.data.server_id,
    );

    fs::write(&config_path, &config_content).map_err(|e| format!("Failed to write config: {e}"))?;
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("Failed to set config permissions: {e}"))?;

    info!("Config written to {}", config_path.display());
    info!("Private key stored at {}", private_key_path.display());
    info!(
        "Enrollment complete. Start the agent with: icefall-agent run --config {}",
        config_path.display()
    );

    Ok(())
}
