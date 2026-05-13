use std::path::Path;
use std::process::Command;

use super::utilities::*;

const MANIFEST_FILE: &str = "manifest.json";

pub async fn import(from: &str, dry_run: bool) {
    let is_s3 = from.starts_with("s3://") || from.starts_with("r2://");
    let local_path = if is_s3 {
        let local = "icefall-import-temp.tar.gz".to_string();
        println!("  Downloading from {from}...");
        let s3_url = from.replacen("r2://", "s3://", 1);
        let result = Command::new("aws")
            .args(["s3", "cp", &s3_url, &local])
            .status();
        match result {
            Ok(status) if status.success() => {
                let size = file_size(Path::new(&local));
                println!("  ✓ Downloaded ({})", format_bytes(size));
            }
            Ok(_) => {
                eprintln!("  ✗ S3 download failed. Make sure `aws` CLI is configured.");
                std::process::exit(1);
            }
            Err(_) => {
                eprintln!("  ✗ `aws` CLI not found. Install it to import from S3/R2.");
                std::process::exit(1);
            }
        }
        local
    } else {
        from.to_string()
    };

    let archive_path = Path::new(&local_path);
    if !archive_path.exists() {
        eprintln!("Archive not found: {from}");
        std::process::exit(1);
    }

    let checksum_path = format!("{local_path}.sha256");
    if Path::new(&checksum_path).exists() {
        print!("  Verifying checksum...");
        let expected = std::fs::read_to_string(&checksum_path)
            .unwrap_or_default()
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        let actual = sha256_file(archive_path);
        if expected == actual {
            println!(" ✓ valid");
        } else {
            println!(" ✗ MISMATCH");
            eprintln!("Expected: {expected}");
            eprintln!("Got:      {actual}");
            eprintln!("The archive may be corrupted or tampered with.");
            std::process::exit(1);
        }
    }

    let data_dir =
        std::env::var("ICEFALL_DATA_DIR").unwrap_or_else(|_| "/var/lib/icefall".to_string());
    let data_path = Path::new(&data_dir);

    let temp_dir = tempfile::TempDir::new().unwrap_or_else(|e| {
        eprintln!("Failed to create temp directory: {e}");
        std::process::exit(1);
    });
    let staging = temp_dir.path();

    println!("  Extracting archive...");
    let file = std::fs::File::open(archive_path).unwrap_or_else(|e| {
        eprintln!("Failed to open archive: {e}");
        std::process::exit(1);
    });
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(staging).unwrap_or_else(|e| {
        eprintln!("Failed to extract archive: {e}");
        std::process::exit(1);
    });

    let manifest_path = staging.join(MANIFEST_FILE);
    if manifest_path.exists() {
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap_or_default())
                .unwrap_or_default();
        let version = manifest
            .get("icefall_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let exported_at = manifest
            .get("exported_at")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        println!();
        println!("Archive manifest:");
        println!("  Icefall version: {version}");
        println!("  Exported at:     {exported_at}");
        if let Some(contents) = manifest.get("contents") {
            let dumps = contents
                .get("db_dumps")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let vols = contents
                .get("volumes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            println!("  Database dumps:  {dumps}");
            println!("  Docker volumes:  {vols}");
        }
        println!();
    }

    if dry_run {
        println!("Dry run — no files restored.");
        return;
    }

    if is_daemon_running() {
        println!("⚠ Stopping the Icefall daemon before import...");
        let _ = Command::new("systemctl").args(["stop", "icefall"]).output();
    }

    println!("Importing full Icefall instance...");
    println!();

    println!("  [1/6] Restoring SQLite database...");
    std::fs::create_dir_all(data_path).ok();
    let db_src = staging.join("icefall.db");
    if db_src.exists() {
        copy_or_exit(&db_src, &data_path.join("icefall.db"));
        println!("    ✓ Database restored");
    } else {
        println!("    ⚠ No database in archive");
    }

    println!("  [2/6] Restoring configuration...");
    let config_src = staging.join("config.toml");
    if config_src.exists() {
        std::fs::create_dir_all("/etc/icefall").ok();
        copy_or_exit(&config_src, Path::new("/etc/icefall/config.toml"));
        println!("    ✓ Config restored (encryption key intact)");
    }

    println!("  [3/6] Restoring Docker volumes...");
    let volumes_src = staging.join("volumes");
    if volumes_src.exists() {
        import_docker_volumes(&volumes_src);
    } else {
        println!("    No volume snapshots found");
    }

    println!("  [4/6] Restoring logs and backups...");
    let logs_src = staging.join("logs");
    if logs_src.exists() {
        copy_dir_recursive(&logs_src, &data_path.join("logs"));
    }
    let backups_src = staging.join("backups");
    if backups_src.exists() {
        copy_dir_recursive(&backups_src, &data_path.join("backups"));
    }

    println!("  [5/6] Staging database dumps...");
    let dumps_src = staging.join("db-dumps");
    if dumps_src.exists() {
        let dest = data_path.join("migration-dumps");
        copy_dir_recursive(&dumps_src, &dest);
        println!("    Dumps staged to {}", dest.display());
    }

    println!("  [6/6] Starting Icefall daemon...");
    let _ = Command::new("systemctl")
        .args(["start", "icefall"])
        .output();

    println!();
    println!("Import complete!");
    println!();
    println!("The daemon is starting. It will:");
    println!("  - Read all app configs from the restored database");
    println!("  - Re-issue SSL certificates via Caddy on first request");
    println!();
    println!("To redeploy all apps (rebuilds container images):");
    println!("  icefall apps list");
    println!("  # For each app, trigger a deploy from the dashboard or CLI");
    println!();

    let dumps_dest = data_path.join("migration-dumps");
    if dumps_dest.exists() {
        println!("Database dumps are staged at: {}", dumps_dest.display());
        println!("After managed database containers start, restore them:");
        if let Ok(entries) = std::fs::read_dir(&dumps_dest) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let container = name.split('.').next().unwrap_or(&name);
                if name.ends_with(".sql.gz") {
                    println!(
                        "  zcat {}/{name} | docker exec -i {container} psql -U icefall",
                        dumps_dest.display()
                    );
                } else if name.ends_with(".rdb") {
                    println!("  docker cp {}/{name} {container}:/data/dump.rdb && docker restart {container}", dumps_dest.display());
                }
            }
        }
    }

    if is_s3 {
        let _ = std::fs::remove_file(&local_path);
    }
}
