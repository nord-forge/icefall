use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use crate::db::Database;

use super::execution::do_backup_sync;

pub(super) async fn run_instance_backup(db: &Arc<dyn Database>) -> Result<String, String> {
    let data_dir =
        std::env::var("ICEFALL_DATA_DIR").unwrap_or_else(|_| "/var/lib/icefall".to_string());
    let data_path = Path::new(&data_dir);

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("instance-backup-{timestamp}.tar.gz");
    let s3_key = format!("instance-backups/{timestamp}.tar.gz");

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

async fn do_backup(data_path: &Path, filename: &str, s3_key: &str) -> Result<i64, String> {
    let data_path = data_path.to_path_buf();
    let filename = filename.to_string();
    let s3_key = s3_key.to_string();

    tokio::task::spawn_blocking(move || do_backup_sync(&data_path, &filename, &s3_key))
        .await
        .map_err(|e| format!("backup task failed: {e}"))?
}

pub(super) async fn cleanup_old_backups(db: &Arc<dyn Database>, retention_count: i64) {
    let Ok(records) = db.list_instance_backup_history(1000).await else {
        return;
    };

    let completed: Vec<_> = records.iter().filter(|r| r.status == "completed").collect();
    if completed.len() as i64 <= retention_count {
        return;
    }

    let to_delete = &completed[retention_count as usize..];
    for record in to_delete {
        if let Some(ref s3_key) = record.s3_key {
            if let Ok(bucket) = std::env::var("ICEFALL_BACKUP_S3_BUCKET") {
                let s3_url = format!("s3://{bucket}/{s3_key}");
                let _ = Command::new("aws").args(["s3", "rm", &s3_url]).output();
            }
        }

        let local_dir = Path::new(
            &std::env::var("ICEFALL_DATA_DIR").unwrap_or_else(|_| "/var/lib/icefall".to_string()),
        )
        .join("instance-backups");
        let local_file = local_dir.join(&record.filename);
        let _ = std::fs::remove_file(local_file);

        let _ = db.delete_instance_backup_record(&record.id).await;
    }
}
