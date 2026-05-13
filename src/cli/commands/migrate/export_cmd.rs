use std::io::{self, Write as _};
use std::path::Path;
use std::process::Command;

use super::utilities::*;

const MANIFEST_FILE: &str = "manifest.json";

pub async fn export(output: &str, dry_run: bool) {
    let is_s3 = output.starts_with("s3://") || output.starts_with("r2://");
    let local_output = if is_s3 {
        "icefall-export-temp.tar.gz".to_string()
    } else {
        output.to_string()
    };
    let output_path = Path::new(&local_output);
    let data_dir =
        std::env::var("ICEFALL_DATA_DIR").unwrap_or_else(|_| "/var/lib/icefall".to_string());
    let data_path = Path::new(&data_dir);

    if !data_path.exists() {
        eprintln!("Data directory not found: {data_dir}");
        std::process::exit(1);
    }

    let db_size = file_size(&data_path.join("icefall.db"));
    let logs_size = dir_size(&data_path.join("logs"));
    let backups_size = dir_size(&data_path.join("backups"));
    let volumes = list_icefall_volumes();
    let db_containers = list_icefall_db_containers();

    println!("Export summary:");
    println!("  SQLite database:     {}", format_bytes(db_size));
    println!("  Log files:           {}", format_bytes(logs_size));
    println!("  Backup archives:     {}", format_bytes(backups_size));
    println!("  Docker volumes:      {} volume(s)", volumes.len());
    println!(
        "  Managed databases:   {} container(s)",
        db_containers.len()
    );
    println!(
        "  Estimated total:     {} (before compression)",
        format_bytes(db_size + logs_size + backups_size)
    );
    println!();

    if dry_run {
        println!("Dry run — no files written.");
        return;
    }

    if is_daemon_running() {
        println!("⚠ The Icefall daemon is running. For a fully consistent export,");
        println!("  consider stopping it first: systemctl stop icefall");
        print!("  Continue anyway? [y/N] ");
        io::stdout().flush().ok();
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).ok();
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Export cancelled.");
            return;
        }
    }

    println!("Exporting full Icefall instance...");
    println!();

    let temp_dir = tempfile::TempDir::new().unwrap_or_else(|e| {
        eprintln!("Failed to create temp directory: {e}");
        std::process::exit(1);
    });
    let staging = temp_dir.path();

    println!("  [1/8] Exporting SQLite database...");
    let db_path = data_path.join("icefall.db");
    if db_path.exists() {
        let _ = Command::new("sqlite3")
            .arg(&db_path)
            .arg("PRAGMA wal_checkpoint(TRUNCATE);")
            .output();
        copy_or_exit(&db_path, &staging.join("icefall.db"));
    }

    println!("  [2/8] Exporting configuration (includes encryption key)...");
    let config_path = find_config();
    if let Some(ref path) = config_path {
        copy_or_exit(Path::new(path), &staging.join("config.toml"));
        println!("    ⚠ config.toml contains your encryption key — keep this archive secure");
    }

    println!("  [3/8] Running fresh database dumps...");
    let dumps_dir = staging.join("db-dumps");
    std::fs::create_dir_all(&dumps_dir).ok();
    export_managed_database_dumps(&dumps_dir);

    println!("  [4/8] Exporting Docker volume data...");
    let volumes_dir = staging.join("volumes");
    std::fs::create_dir_all(&volumes_dir).ok();
    export_docker_volumes(&volumes_dir);

    println!("  [5/8] Exporting log files...");
    let logs_dir = data_path.join("logs");
    if logs_dir.exists() {
        copy_dir_recursive(&logs_dir, &staging.join("logs"));
    }

    println!("  [6/8] Exporting backup archives...");
    let backups_dir = data_path.join("backups");
    if backups_dir.exists() {
        copy_dir_recursive(&backups_dir, &staging.join("backups"));
    }

    println!("  [7/8] Writing manifest...");
    let manifest = serde_json::json!({
        "icefall_version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "contents": {
            "database": db_path.exists(),
            "config": config_path.is_some(),
            "db_dumps": db_containers.len(),
            "volumes": volumes.len(),
            "has_logs": logs_dir.exists(),
            "has_backups": backups_dir.exists(),
        }
    });
    std::fs::write(
        staging.join(MANIFEST_FILE),
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    )
    .ok();

    println!("  [8/8] Creating archive...");
    let file = std::fs::File::create(output_path).unwrap_or_else(|e| {
        eprintln!("Failed to create output file: {e}");
        std::process::exit(1);
    });
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);

    archive.append_dir_all(".", staging).unwrap_or_else(|e| {
        eprintln!("Failed to create archive: {e}");
        std::process::exit(1);
    });

    let encoder = archive.into_inner().unwrap_or_else(|e| {
        eprintln!("Failed to finalize archive: {e}");
        std::process::exit(1);
    });
    encoder.finish().unwrap_or_else(|e| {
        eprintln!("Failed to compress archive: {e}");
        std::process::exit(1);
    });

    let checksum = sha256_file(output_path);
    let checksum_path = format!("{local_output}.sha256");
    std::fs::write(&checksum_path, format!("{checksum}  {local_output}\n")).ok();

    let size = std::fs::metadata(output_path).map_or(0, |m| m.len());

    if is_s3 {
        println!();
        println!("  Uploading to {output}...");
        let s3_url = output.replacen("r2://", "s3://", 1);
        let upload_result = Command::new("aws")
            .args(["s3", "cp", &local_output, &s3_url])
            .status();

        match upload_result {
            Ok(status) if status.success() => {
                println!("  ✓ Uploaded to {output}");
                let _ = std::fs::remove_file(&local_output);
                let _ = std::fs::remove_file(&checksum_path);
            }
            Ok(_) => {
                eprintln!("  ✗ S3 upload failed. Archive saved locally as {local_output}");
                eprintln!("    Make sure `aws` CLI is installed and configured.");
                eprintln!("    For R2: aws configure --profile r2, then set AWS_PROFILE=r2");
            }
            Err(_) => {
                eprintln!("  ✗ `aws` CLI not found. Install it to enable S3/R2 uploads.");
                eprintln!("    Archive saved locally as {local_output}");
            }
        }
    }

    println!();
    println!(
        "Export complete: {} ({})",
        if is_s3 { output } else { &local_output },
        format_bytes(size)
    );
    if !is_s3 {
        println!("Checksum:        {checksum_path}");
    }
    println!();
    println!("Included:");
    println!("  ✓ SQLite database (apps, users, sessions, env vars)");
    println!("  ✓ Configuration with encryption key");
    println!(
        "  ✓ Fresh database dumps ({} database(s))",
        db_containers.len()
    );
    println!("  ✓ Docker volume snapshots ({} volume(s))", volumes.len());
    println!("  ✓ Log files and backup archives");
    println!();
    println!("Not included (auto-handled on new server):");
    println!("  - Container images (re-pulled on deploy)");
    println!("  - SSL certificates (re-issued by Caddy)");
}
