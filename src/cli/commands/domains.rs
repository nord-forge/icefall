use crate::cli::client::CliClient;

pub async fn add(app: &str, domain: &str) {
    let client = CliClient::new_or_exit();

    match client
        .post::<serde_json::Value>(
            &format!("/apps/{app}/domains"),
            &serde_json::json!({"domain": domain}),
        )
        .await
    {
        Ok(resp) => {
            println!("Domain {domain} added.");
            if let Some(dns) = resp.get("dns_instructions") {
                println!("\nDNS Instructions:");
                if let Some(ip) = dns.get("value").and_then(|v| v.as_str()) {
                    println!("  A  {domain}  →  {ip}");
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

pub async fn list(app: &str) {
    let client = CliClient::new_or_exit();

    match client
        .get::<serde_json::Value>(&format!("/apps/{app}/domains"))
        .await
    {
        Ok(resp) => {
            let domains = resp
                .get("data")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if domains.is_empty() {
                println!("No domains configured.");
                return;
            }
            println!("{:<30} {:<10} VERIFIED", "DOMAIN", "SSL");
            println!("{}", "-".repeat(50));
            for d in &domains {
                let domain = d.get("domain").and_then(|v| v.as_str()).unwrap_or("?");
                let ssl = d.get("ssl_status").and_then(|v| v.as_str()).unwrap_or("-");
                let verified = d
                    .get("verified")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                println!(
                    "{:<30} {:<10} {}",
                    domain,
                    ssl,
                    if verified { "yes" } else { "no" }
                );
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
