use crate::cli::client::CliClient;

pub async fn status() {
    let client = CliClient::new_or_exit();

    match client.get::<serde_json::Value>("/server/status").await {
        Ok(resp) => {
            let version = resp.get("version").and_then(|v| v.as_str()).unwrap_or("?");
            let cpu = resp
                .get("cpu_percent")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let mem_used = resp
                .get("memory_used_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let mem_total = resp
                .get("memory_total_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let disk_used = resp
                .get("disk_used_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let disk_total = resp
                .get("disk_total_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);

            println!("Icefall Server v{version}");
            println!();
            println!("  CPU:    {:.1}%", cpu);
            println!(
                "  Memory: {:.1} / {:.1} GB",
                mem_used as f64 / 1e9,
                mem_total as f64 / 1e9
            );
            println!(
                "  Disk:   {:.1} / {:.1} GB",
                disk_used as f64 / 1e9,
                disk_total as f64 / 1e9
            );
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
