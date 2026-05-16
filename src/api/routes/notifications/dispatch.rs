use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;

pub async fn dispatch_notification(
    channel_type: &str,
    config: &str,
    event: &str,
    summary: &str,
    details: &serde_json::Value,
) -> Result<(), String> {
    match channel_type {
        "webhook" => dispatch_webhook(config, event, summary, details).await,
        "smtp" => dispatch_smtp(config, event, summary, details).await,
        "ntfy" => dispatch_ntfy(config, event, summary, details).await,
        "plunk" => dispatch_plunk(config, event, summary, details).await,
        "slack" => dispatch_slack(config, event, summary, details).await,
        "discord" => dispatch_discord(config, event, summary, details).await,
        _ => Err(format!("unknown channel type: {channel_type}")),
    }
}

async fn dispatch_webhook(
    config: &str,
    event: &str,
    summary: &str,
    details: &serde_json::Value,
) -> Result<(), String> {
    let parsed: serde_json::Value = serde_json::from_str(config).map_err(|e| e.to_string())?;
    let url = parsed
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("webhook config missing 'url'")?;

    let payload = serde_json::json!({
        "event": event,
        "summary": summary,
        "details": details,
        "timestamp": crate::db::models::now_iso8601(),
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("webhook returned {}", resp.status()))
    }
}

async fn dispatch_slack(
    config: &str,
    event: &str,
    summary: &str,
    details: &serde_json::Value,
) -> Result<(), String> {
    let parsed: serde_json::Value = serde_json::from_str(config).map_err(|e| e.to_string())?;
    let webhook_url = parsed
        .get("webhook_url")
        .and_then(|v| v.as_str())
        .ok_or("slack config missing 'webhook_url'")?;

    let color = event_color_hex(event);
    let timestamp = crate::db::models::now_iso8601();
    let app_name = details
        .get("app_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let payload = serde_json::json!({
        "blocks": [
            {
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": format!("{summary}: {app_name}")
                }
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!(
                        "*Event:* {event}\n*App:* {app_name}\n*Time:* {timestamp}"
                    )
                }
            }
        ],
        "attachments": [{ "color": color }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(webhook_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("slack webhook returned {}", resp.status()))
    }
}

async fn dispatch_discord(
    config: &str,
    event: &str,
    summary: &str,
    details: &serde_json::Value,
) -> Result<(), String> {
    let parsed: serde_json::Value = serde_json::from_str(config).map_err(|e| e.to_string())?;
    let webhook_url = parsed
        .get("webhook_url")
        .and_then(|v| v.as_str())
        .ok_or("discord config missing 'webhook_url'")?;

    let color = event_color_discord(event);
    let timestamp = crate::db::models::now_iso8601();
    let app_name = details
        .get("app_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let payload = serde_json::json!({
        "embeds": [{
            "title": summary,
            "description": format!(
                "**App:** {app_name}\n**Event:** {event}"
            ),
            "color": color,
            "timestamp": timestamp,
            "footer": { "text": "Icefall" }
        }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(webhook_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() || resp.status().as_u16() == 204 {
        Ok(())
    } else {
        Err(format!("discord webhook returned {}", resp.status()))
    }
}

async fn dispatch_plunk(
    config: &str,
    event: &str,
    summary: &str,
    details: &serde_json::Value,
) -> Result<(), String> {
    let parsed: serde_json::Value = serde_json::from_str(config).map_err(|e| e.to_string())?;
    let api_key = parsed
        .get("api_key")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let to_email = parsed
        .get("to_email")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if api_key.is_empty() || to_email.is_empty() {
        return Err("Plunk: missing api_key or to_email".to_string());
    }

    let app_name = details
        .get("app_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let body = serde_json::json!({
        "to": to_email,
        "subject": format!("[Icefall] {event}: {summary}"),
        "body": format!(
            "Event: {event}\nSummary: {summary}\nApp: {app_name}\n\nDetails:\n{details_pretty}\n\nTimestamp: {ts}",
            details_pretty = serde_json::to_string_pretty(details).unwrap_or_default(),
            ts = crate::db::models::now_iso8601(),
        ),
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.useplunk.com/v1/send")
        .bearer_auth(api_key)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Plunk API error: {e}"))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Plunk returned error: {text}"));
    }

    tracing::info!("Plunk notification sent: [{event}] {summary}");
    Ok(())
}

async fn dispatch_ntfy(
    config: &str,
    event: &str,
    summary: &str,
    details: &serde_json::Value,
) -> Result<(), String> {
    let parsed: serde_json::Value = serde_json::from_str(config).map_err(|e| e.to_string())?;
    let topic = parsed
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let server = parsed
        .get("server")
        .and_then(|v| v.as_str())
        .unwrap_or("https://ntfy.sh");

    if topic.is_empty() {
        return Err("ntfy: topic is required".to_string());
    }

    let url = format!("{}/{}", server.trim_end_matches('/'), topic);

    let priority = match event {
        "deploy.failed" | "health.down" | "backup.failure" => "high",
        "deploy.success" | "health.recovered" | "backup.success" => "default",
        _ => "default",
    };

    let tags = match event {
        "deploy.success" => "white_check_mark",
        "deploy.failed" => "x",
        "health.down" => "warning",
        "health.recovered" => "green_heart",
        "backup.success" => "package",
        "backup.failure" => "rotating_light",
        _ => "bell",
    };

    let app_name = details
        .get("app_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let body = format!("{summary}\nApp: {app_name}");

    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .header("Title", format!("[Icefall] {event}"))
        .header("Priority", priority)
        .header("Tags", tags)
        .body(body)
        .timeout(std::time::Duration::from_secs(10));

    if let Some(token) = parsed.get("token").and_then(|v| v.as_str()) {
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }
    }

    let resp = req.send().await.map_err(|e| format!("ntfy error: {e}"))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("ntfy returned error: {text}"));
    }

    tracing::info!("ntfy notification sent to {topic}: [{event}] {summary}");
    Ok(())
}

fn event_color_hex(event: &str) -> &'static str {
    match event {
        "deploy.success" | "health.recovered" | "backup.success" => "#00c853",
        "deploy.failure" | "health.down" | "backup.failure" => "#ff1744",
        "health.auto_restart" => "#ffc107",
        _ => "#9e9e9e",
    }
}

fn event_color_discord(event: &str) -> u32 {
    match event {
        "deploy.success" | "health.recovered" | "backup.success" => 0x00c853,
        "deploy.failure" | "health.down" | "backup.failure" => 0xff1744,
        "health.auto_restart" => 0xffc107,
        _ => 0x9e9e9e,
    }
}

#[derive(Deserialize)]
struct SmtpConfig {
    host: String,
    port: Option<u16>,
    username: String,
    password: String,
    from: String,
    to: String,
    tls: Option<String>,
}

async fn dispatch_smtp(
    config: &str,
    event: &str,
    summary: &str,
    details: &serde_json::Value,
) -> Result<(), String> {
    let smtp: SmtpConfig =
        serde_json::from_str(config).map_err(|e| format!("invalid SMTP config: {e}"))?;

    let tls_mode = smtp.tls.as_deref().unwrap_or("starttls");

    let from_mailbox: Mailbox = smtp
        .from
        .parse()
        .map_err(|e| format!("invalid 'from' address '{}': {e}", smtp.from))?;

    let to_mailbox: Mailbox = smtp
        .to
        .parse()
        .map_err(|e| format!("invalid 'to' address '{}': {e}", smtp.to))?;

    let subject = format!("[Icefall] {event}: {summary}");
    let body = format!(
        "Event: {event}\nSummary: {summary}\n\nDetails:\n{details_pretty}\n\nTimestamp: {ts}",
        details_pretty = serde_json::to_string_pretty(details).unwrap_or_default(),
        ts = crate::db::models::now_iso8601(),
    );

    let email = Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(subject)
        .body(body)
        .map_err(|e| format!("failed to build email: {e}"))?;

    let creds = Credentials::new(smtp.username.clone(), smtp.password.clone());

    let default_port = match tls_mode {
        "tls" => 465,
        "none" => 25,
        _ => 587,
    };
    let port = smtp.port.unwrap_or(default_port);

    let mailer = match tls_mode {
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)
            .map_err(|e| format!("SMTP relay error: {e}"))?
            .port(port)
            .credentials(creds)
            .build(),
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host)
            .port(port)
            .credentials(creds)
            .build(),
        _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)
            .map_err(|e| format!("SMTP STARTTLS relay error: {e}"))?
            .port(port)
            .credentials(creds)
            .build(),
    };

    mailer.send(email).await.map_err(|e| {
        tracing::error!("SMTP send failed for [{event}]: {e}");
        format!("SMTP send failed: {e}")
    })?;

    tracing::info!("SMTP notification sent: [{event}] {summary}");
    Ok(())
}
