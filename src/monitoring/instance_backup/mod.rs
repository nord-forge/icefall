mod execution;
mod orchestration;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::db::Database;

use orchestration::run_instance_backup;

const CHECK_INTERVAL_SECS: u64 = 600;

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

fn schedule_to_interval_secs(schedule: &str) -> u64 {
    match schedule {
        "daily" => 86400,
        "weekly" => 604800,
        "monthly" => 2592000,
        _ => 86400,
    }
}

pub fn spawn_instance_backup_scheduler(db: Arc<dyn Database>, handle: Arc<InstanceBackupHandle>) {
    tokio::spawn(async move {
        let mut last_backup: Option<chrono::DateTime<chrono::Utc>> = None;

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

            let Ok(Some(config)) = db.get_instance_backup_config().await else {
                continue;
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
                        last_backup = Some(chrono::Utc::now());
                    }
                }
            }
        }
    });
}
