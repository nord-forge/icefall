use std::sync::Arc;
use std::time::Duration;

use crate::db::Database;
use crate::events::{EventBus, EventType};

const MIN_INTERVAL_HOURS: u64 = 1;

/// Spawn a background task that checks SSL certificates on a configurable interval.
/// `interval_hours` defaults to 24. Minimum is 1 hour.
pub fn spawn_ssl_checker(db: Arc<dyn Database>, event_bus: Arc<EventBus>, interval_hours: u64) {
    let interval_secs = interval_hours.max(MIN_INTERVAL_HOURS) * 3600;

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;

        loop {
            check_all_certificates(&db, &event_bus).await;
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

async fn check_all_certificates(db: &Arc<dyn Database>, event_bus: &Arc<EventBus>) {
    let Ok(domains) = db.list_all_domains().await else {
        return;
    };

    for domain in &domains {
        match check_certificate_expiry(&domain.domain).await {
            Ok((issuer, expires_at)) => {
                let _ = db
                    .update_domain_ssl_info(&domain.id, Some(&issuer), Some(&expires_at))
                    .await;

                if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(&expires_at) {
                    let days_left =
                        (expiry.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days();

                    let event = if days_left <= 0 {
                        Some("ssl.expired")
                    } else if days_left <= 7 {
                        Some("ssl.critical")
                    } else if days_left <= 14 {
                        Some("ssl.expiring")
                    } else {
                        None
                    };

                    if let Some(event_name) = event {
                        event_bus.emit(
                            EventType::DiskAlert,
                            Some(&domain.app_id),
                            Some(&domain.id),
                            serde_json::json!({
                                "event": event_name,
                                "domain": domain.domain,
                                "expires_at": expires_at,
                                "days_until_expiry": days_left,
                                "issuer": issuer,
                            }),
                        );
                    }
                }
            }
            Err(e) => {
                tracing::debug!("SSL check failed for {}: {e}", domain.domain);
                let _ = db.update_domain_ssl_info(&domain.id, None, None).await;
            }
        }
    }
}

async fn check_certificate_expiry(domain: &str) -> Result<(String, String), String> {
    let combined = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "echo | openssl s_client -connect {domain}:443 -servername {domain} 2>/dev/null | openssl x509 -noout -issuer -enddate 2>/dev/null"
        ))
        .output()
        .await
        .map_err(|e| format!("certificate check failed: {e}"))?;

    let text = String::from_utf8_lossy(&combined.stdout);
    let mut issuer = String::new();
    let mut expiry = String::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("issuer=") {
            issuer = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("notAfter=") {
            expiry = rest.trim().to_string();
        }
    }

    if expiry.is_empty() {
        return Err("could not read certificate expiry".to_string());
    }

    // Parse openssl date format "Jan  1 00:00:00 2026 GMT" to ISO 8601
    let parsed = chrono::NaiveDateTime::parse_from_str(&expiry, "%b %d %H:%M:%S %Y %Z")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&expiry, "%b  %d %H:%M:%S %Y %Z"))
        .map_err(|e| format!("date parse error: {e}"))?;

    let iso = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(parsed, chrono::Utc)
        .to_rfc3339();

    Ok((issuer, iso))
}
