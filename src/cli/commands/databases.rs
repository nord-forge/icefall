use crate::cli::client::CliClient;

pub async fn create(db_type: &str) {
    let client = CliClient::new_or_exit();

    let name = format!("{db_type}-{}", &uuid::Uuid::now_v7().to_string()[..8]);
    println!("Creating {db_type} database '{name}'...");

    match client
        .post::<serde_json::Value>(
            "/databases",
            &serde_json::json!({"name": name, "db_type": db_type}),
        )
        .await
    {
        Ok(resp) => {
            if let Some(data) = resp.get("data") {
                let conn = data
                    .get("connection_string")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                println!("Database created: {name}");
                if !conn.is_empty() {
                    println!("Connection: {conn}");
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

pub async fn list() {
    let client = CliClient::new_or_exit();

    match client.get::<serde_json::Value>("/databases").await {
        Ok(resp) => {
            let dbs = resp
                .get("data")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if dbs.is_empty() {
                println!("No databases.");
                return;
            }
            println!("{:<20} {:<12} CREATED", "NAME", "TYPE");
            println!("{}", "-".repeat(50));
            for db in &dbs {
                let name = db.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let db_type = db.get("db_type").and_then(|v| v.as_str()).unwrap_or("-");
                let created = db.get("created_at").and_then(|v| v.as_str()).unwrap_or("-");
                println!(
                    "{:<20} {:<12} {}",
                    name,
                    db_type,
                    &created[..19.min(created.len())]
                );
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

pub async fn backup(db: &str) {
    let client = CliClient::new_or_exit();

    println!("Triggering backup for {db}...");
    match client
        .post::<serde_json::Value>(&format!("/databases/{db}/backup"), &serde_json::json!({}))
        .await
    {
        Ok(resp) => {
            if let Some(data) = resp.get("data") {
                let filename = data.get("filename").and_then(|v| v.as_str()).unwrap_or("?");
                let size = data.get("size_bytes").and_then(serde_json::Value::as_u64).unwrap_or(0);
                println!("Backup complete: {filename} ({size} bytes)");
            }
        }
        Err(e) => {
            eprintln!("Backup failed: {e}");
            std::process::exit(1);
        }
    }
}
