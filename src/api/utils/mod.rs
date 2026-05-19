pub mod url_guard;

use std::net::IpAddr;

pub fn hash_password(password: &str) -> Result<String, String> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub async fn detect_server_ip() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp = client.get("https://api.ipify.org").send().await.ok()?;
    let ip = resp.text().await.ok()?;
    let trimmed = ip.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub async fn resolve_domain(domain: &str) -> Result<Vec<IpAddr>, String> {
    let lookup = format!("{domain}:443");
    let result = tokio::net::lookup_host(&lookup).await;
    match result {
        Ok(addrs) => {
            let ips: Vec<IpAddr> = addrs.map(|a| a.ip()).collect();
            if ips.is_empty() {
                Err("no DNS records found".to_string())
            } else {
                Ok(ips)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub async fn check_dns_points_to(domain: &str, expected_ip: Option<&str>) -> bool {
    let Some(expected) = expected_ip else {
        return false;
    };
    resolve_domain(domain)
        .await
        .is_ok_and(|ips| ips.iter().any(|ip| ip.to_string() == expected))
}
