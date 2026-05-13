use std::path::Path;
use std::process::Command;

pub(super) fn do_backup_sync(
    data_path: &Path,
    filename: &str,
    s3_key: &str,
) -> Result<i64, String> {
    let temp_dir =
        tempfile::TempDir::new().map_err(|e| format!("failed to create temp dir: {e}"))?;
    let staging = temp_dir.path();

    let db_path = data_path.join("icefall.db");
    if db_path.exists() {
        let _ = Command::new("sqlite3")
            .arg(&db_path)
            .arg("PRAGMA wal_checkpoint(TRUNCATE);")
            .output();
        std::fs::copy(&db_path, staging.join("icefall.db"))
            .map_err(|e| format!("failed to copy database: {e}"))?;
    }

    let config_paths = [
        "/etc/icefall/config.toml".to_string(),
        dirs::config_dir()
            .unwrap_or_default()
            .join("icefall/config.toml")
            .to_string_lossy()
            .to_string(),
    ];
    for p in &config_paths {
        if Path::new(p).exists() {
            std::fs::copy(p, staging.join("config.toml")).ok();
            break;
        }
    }

    let dumps_dir = staging.join("db-dumps");
    std::fs::create_dir_all(&dumps_dir).ok();
    run_managed_db_dumps(&dumps_dir);

    let volumes_dir = staging.join("volumes");
    std::fs::create_dir_all(&volumes_dir).ok();
    export_docker_volumes(&volumes_dir);

    let logs_dir = data_path.join("logs");
    if logs_dir.exists() {
        copy_dir_recursive(&logs_dir, &staging.join("logs"));
    }

    let backups_dir = data_path.join("backups");
    if backups_dir.exists() {
        copy_dir_recursive(&backups_dir, &staging.join("backups"));
    }

    let manifest = serde_json::json!({
        "icefall_version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "backup_type": "scheduled_instance",
    });
    std::fs::write(
        staging.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    )
    .ok();

    let archive_path = temp_dir.path().join(filename);
    let file = std::fs::File::create(&archive_path)
        .map_err(|e| format!("failed to create archive file: {e}"))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive
        .append_dir_all(".", staging)
        .map_err(|e| format!("failed to build archive: {e}"))?;
    let encoder = archive
        .into_inner()
        .map_err(|e| format!("failed to finalize archive: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("failed to compress archive: {e}"))?;

    let size_bytes = std::fs::metadata(&archive_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    let s3_bucket = std::env::var("ICEFALL_BACKUP_S3_BUCKET").ok();
    if let Some(bucket) = s3_bucket {
        let s3_url = format!("s3://{bucket}/{s3_key}");
        let upload_result = Command::new("aws")
            .args(["s3", "cp", &archive_path.to_string_lossy(), &s3_url])
            .output();

        match upload_result {
            Ok(out) if out.status.success() => {
                tracing::info!("Instance backup uploaded to {s3_url}");
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(format!("S3 upload failed: {stderr}"));
            }
            Err(e) => {
                return Err(format!("aws CLI not available: {e}"));
            }
        }
    } else {
        let local_dir = Path::new(
            &std::env::var("ICEFALL_DATA_DIR").unwrap_or_else(|_| "/var/lib/icefall".to_string()),
        )
        .join("instance-backups");
        std::fs::create_dir_all(&local_dir).ok();
        let local_dest = local_dir.join(filename);
        std::fs::copy(&archive_path, &local_dest)
            .map_err(|e| format!("failed to copy backup to local storage: {e}"))?;
        tracing::info!("Instance backup stored locally at {}", local_dest.display());
    }

    Ok(size_bytes)
}

fn run_managed_db_dumps(dumps_dir: &Path) {
    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            "label=icefall.managed-db=true",
            "--format",
            "{{.Names}}\t{{.Image}}",
        ])
        .output();

    let containers: Vec<(String, String)> = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    };

    for (name, db_type) in &containers {
        let dump_file = dumps_dir.join(format!("{name}.sql.gz"));
        let dump_cmd = match db_type.as_str() {
            t if t.contains("postgres") => {
                format!(
                    "docker exec {name} pg_dumpall -U icefall | gzip > {}",
                    dump_file.display()
                )
            }
            t if t.contains("mysql") => {
                format!(
                    "docker exec {name} mysqldump -u icefall --all-databases | gzip > {}",
                    dump_file.display()
                )
            }
            t if t.contains("mongo") => {
                format!(
                    "docker exec {name} mongodump --archive --gzip --username icefall > {}",
                    dump_file.display()
                )
            }
            t if t.contains("redis") => {
                let rdb_file = dumps_dir.join(format!("{name}.rdb"));
                format!(
                    "docker exec {name} redis-cli BGSAVE && sleep 2 && docker cp {name}:/data/dump.rdb {}",
                    rdb_file.display()
                )
            }
            _ => continue,
        };

        let _ = Command::new("sh").arg("-c").arg(&dump_cmd).output();
    }
}

fn export_docker_volumes(volumes_dir: &Path) {
    let output = Command::new("docker")
        .args([
            "volume",
            "ls",
            "--filter",
            "name=icefall-db-",
            "--format",
            "{{.Name}}",
        ])
        .output();

    let volumes: Vec<String> = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    };

    for volume in &volumes {
        let _ = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-v",
                &format!("{volume}:/data"),
                "-v",
                &format!("{}:/backup", volumes_dir.display()),
                "alpine",
                "tar",
                "czf",
                &format!("/backup/{volume}.tar.gz"),
                "-C",
                "/data",
                ".",
            ])
            .output();
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).ok();
    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path);
            } else {
                let _ = std::fs::copy(&src_path, &dst_path);
            }
        }
    }
}
