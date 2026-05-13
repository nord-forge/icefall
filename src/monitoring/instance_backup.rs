use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::db::Database;

/// Interval between schedule checks (10 minutes)
const CHECK_INTERVAL_SECS: u64 = 600;

/// Shared handle for manually triggering instance backups from the API.
pub struct InstanceBackupHandle {
    db: Arc<dyn Database>,
    running: Mutex<bool>,
}

impl InstanceBackupHandle {
    pub fn new(db: Arc<dyn Database>) -> Self {
        Self {
            db,
            running: Mutex::new(false),
        }
    }

    /// Trigger an instance backup. Returns the backup record ID or an error message.
    pub async fn trigger(&self) -> Result<String, String> {
        let mut running = self.running.lock().await;
        if *running {
            return Err("An instance backup is already in progress".to_string());
        }
        *running = true;
        drop(running);

        let result = run_instance_backup(&self.db).await;

        let mut running = self.running.lock().await;
        *running = false;

        result
    }
}

/// Run the full instance backup: create archive, upload to S3, record in DB.
async fn run_instance_backup(db: &Arc<dyn Database>) -> Result<String, String> {
    let data_dir =
        std::env::var("ICEFALL_DATA_DIR").unwrap_or_else(|_| "/var/lib/icefall".to_string());
    let data_path = Path::new(&data_dir);

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("instance-backup-{timestamp}.tar.gz");
    let s3_key = format!("instance-backups/{timestamp}.tar.gz");

    // Create the history record
    let record = db
        .create_instance_backup_record(&filename, Some(&s3_key))
        .await
        .map_err(|e| format!("failed to create backup record: {e}"))?;

    let record_id = record.id.clone();

    match do_backup(data_path, &filename, &s3_key).await {
        Ok(size_bytes) => {
            db.update_instance_backup_record(&record_id, "completed", size_bytes, None)
                .await
                .ok();

            // Clean up old backups based on retention
            if let Ok(Some(config)) = db.get_instance_backup_config().await {
                cleanup_old_backups(db, config.retention_count).await;
            }

            tracing::info!("Instance backup completed: {filename} ({size_bytes} bytes)");
            Ok(record_id)
        }
        Err(e) => {
            db.update_instance_backup_record(&record_id, "failed", 0, Some(&e))
                .await
                .ok();
            tracing::error!("Instance backup failed: {e}");
            Err(e)
        }
    }
}

/// Perform the actual backup to a tar.gz and upload to S3.
async fn do_backup(data_path: &Path, filename: &str, s3_key: &str) -> Result<i64, String> {
    let data_path = data_path.to_path_buf();
    let filename = filename.to_string();
    let s3_key = s3_key.to_string();

    tokio::task::spawn_blocking(move || do_backup_sync(&data_path, &filename, &s3_key))
        .await
        .map_err(|e| format!("backup task failed: {e}"))?
}

fn do_backup_sync(data_path: &Path, filename: &str, s3_key: &str) -> Result<i64, String> {
    let temp_dir =
        tempfile::TempDir::new().map_err(|e| format!("failed to create temp dir: {e}"))?;
    let staging = temp_dir.path();

    // Step 1: SQLite database (with WAL checkpoint)
    let db_path = data_path.join("icefall.db");
    if db_path.exists() {
        let _ = Command::new("sqlite3")
            .arg(&db_path)
            .arg("PRAGMA wal_checkpoint(TRUNCATE);")
            .output();
        std::fs::copy(&db_path, staging.join("icefall.db"))
            .map_err(|e| format!("failed to copy database: {e}"))?;
    }

    // Step 2: Configuration
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

    // Step 3: Fresh database dumps
    let dumps_dir = staging.join("db-dumps");
    std::fs::create_dir_all(&dumps_dir).ok();
    run_managed_db_dumps(&dumps_dir);

    // Step 4: Docker volumes
    let volumes_dir = staging.join("volumes");
    std::fs::create_dir_all(&volumes_dir).ok();
    export_docker_volumes(&volumes_dir);

    // Step 5: Logs
    let logs_dir = data_path.join("logs");
    if logs_dir.exists() {
        copy_dir_recursive(&logs_dir, &staging.join("logs"));
    }

    // Step 6: Backups (existing database backups)
    let backups_dir = data_path.join("backups");
    if backups_dir.exists() {
        copy_dir_recursive(&backups_dir, &staging.join("backups"));
    }

    // Step 7: Manifest
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

    // Step 8: Create tar.gz
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

    // Step 9: Upload to S3 (using aws CLI, same as the export command)
    // Look for configured backup location from the database
    // For now, use aws s3 cp if AWS_* env vars or aws config exist
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
        // Store locally in data_dir/instance-backups/
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

/// Clean up old backups beyond the retention limit.
async fn cleanup_old_backups(db: &Arc<dyn Database>, retention_count: i64) {
    let records = match db.list_instance_backup_history(1000).await {
        Ok(r) => r,
        Err(_) => return,
    };

    // Only count completed backups for retention
    let completed: Vec<_> = records.iter().filter(|r| r.status == "completed").collect();
    if completed.len() as i64 <= retention_count {
        return;
    }

    let to_delete = &completed[retention_count as usize..];
    for record in to_delete {
        // Try to remove from S3
        if let Some(ref s3_key) = record.s3_key {
            if let Ok(bucket) = std::env::var("ICEFALL_BACKUP_S3_BUCKET") {
                let s3_url = format!("s3://{bucket}/{s3_key}");
                let _ = Command::new("aws").args(["s3", "rm", &s3_url]).output();
            }
        }

        // Try to remove local file
        let local_dir = Path::new(
            &std::env::var("ICEFALL_DATA_DIR").unwrap_or_else(|_| "/var/lib/icefall".to_string()),
        )
        .join("instance-backups");
        let local_file = local_dir.join(&record.filename);
        let _ = std::fs::remove_file(local_file);

        // Remove DB record
        let _ = db.delete_instance_backup_record(&record.id).await;
    }
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

/// Convert a human-friendly schedule name to seconds between runs.
fn schedule_to_interval_secs(schedule: &str) -> u64 {
    match schedule {
        "daily" => 86400,
        "weekly" => 604800,
        "monthly" => 2592000, // 30 days
        _ => 86400,
    }
}

/// Spawn the background task that checks the instance backup config and runs backups.
pub fn spawn_instance_backup_scheduler(db: Arc<dyn Database>, handle: Arc<InstanceBackupHandle>) {
    tokio::spawn(async move {
        // Track the last backup time so we know when to fire next
        let mut last_backup: Option<chrono::DateTime<chrono::Utc>> = None;

        // On startup, look at last completed backup to seed the timer
        if let Ok(history) = db.list_instance_backup_history(1).await {
            if let Some(last) = history.first() {
                if last.status == "completed" {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&last.started_at) {
                        last_backup = Some(dt.with_timezone(&chrono::Utc));
                    }
                }
            }
        }

        loop {
            tokio::time::sleep(Duration::from_secs(CHECK_INTERVAL_SECS)).await;

            let config = match db.get_instance_backup_config().await {
                Ok(Some(c)) => c,
                _ => continue,
            };

            if !config.enabled {
                continue;
            }

            let interval = Duration::from_secs(schedule_to_interval_secs(&config.cron_schedule));
            let now = chrono::Utc::now();

            let should_run = match last_backup {
                None => true,
                Some(last) => {
                    let elapsed = now.signed_duration_since(last);
                    elapsed.to_std().unwrap_or(Duration::ZERO) >= interval
                }
            };

            if should_run {
                tracing::info!(
                    "Starting scheduled instance backup (schedule: {})",
                    config.cron_schedule
                );
                match handle.trigger().await {
                    Ok(_) => {
                        last_backup = Some(chrono::Utc::now());
                    }
                    Err(e) => {
                        tracing::error!("Scheduled instance backup failed: {e}");
                        // Still update last_backup to avoid hammering on failure
                        last_backup = Some(chrono::Utc::now());
                    }
                }
            }
        }
    });
}
