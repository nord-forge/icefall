use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::db::Database;
use crate::docker::DockerClient;

pub struct BackupStore {
    base_dir: PathBuf,
}

impl BackupStore {
    pub fn new(data_dir: &Path) -> Self {
        let base_dir = data_dir.join("backups");
        std::fs::create_dir_all(&base_dir).ok();
        Self { base_dir }
    }

    fn db_backup_dir(&self, db_id: &str) -> PathBuf {
        let dir = self.base_dir.join(db_id);
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    pub async fn run_backup(
        &self,
        docker: &DockerClient,
        db_id: &str,
        db_type: &str,
        container_name: &str,
    ) -> Result<BackupInfo, String> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("{db_id}_{timestamp}.sql.gz");
        let backup_dir = self.db_backup_dir(db_id);
        let backup_path = backup_dir.join(&filename);

        let dump_cmd = match db_type {
            "postgres" => "pg_dumpall -U icefall | gzip",
            "mysql" => "mysqldump -u icefall --all-databases | gzip",
            "mariadb" => "mariadb-dump -u icefall --all-databases | gzip",
            "redis" | "keydb" | "valkey" => "redis-cli BGSAVE && sleep 2 && cat /data/dump.rdb | gzip",
            "dragonfly" => "redis-cli BGSAVE && sleep 2 && cat /data/dump.rdb | gzip",
            "mongo" => "mongodump --archive --gzip --username icefall",
            "clickhouse" => "clickhouse-client --query 'SELECT * FROM system.tables FORMAT TabSeparated' | gzip",
            "cockroachdb" => "cockroach dump --insecure defaultdb | gzip",
            "cassandra" => "nodetool snapshot -t backup && tar czf /tmp/cassandra-backup.tar.gz /var/lib/cassandra/data/*/snapshots/backup/ && cat /tmp/cassandra-backup.tar.gz",
            _ => return Err(format!("unsupported db type: {db_type}")),
        };

        let exec = docker
            .inner()
            .create_exec(
                container_name,
                bollard::exec::CreateExecOptions {
                    cmd: Some(vec!["sh", "-c", dump_cmd]),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| format!("exec create failed: {e}"))?;

        let output = docker
            .inner()
            .start_exec(&exec.id, None::<bollard::exec::StartExecOptions>)
            .await
            .map_err(|e| format!("exec start failed: {e}"))?;

        let mut data = Vec::new();
        if let bollard::exec::StartExecResults::Attached { mut output, .. } = output {
            use futures_util::StreamExt;
            while let Some(Ok(chunk)) = output.next().await {
                data.extend_from_slice(&chunk.into_bytes());
            }
        }

        std::fs::write(&backup_path, &data).map_err(|e| format!("failed to write backup: {e}"))?;

        self.cleanup_old(db_id, 7);

        Ok(BackupInfo {
            id: format!("{db_id}_{timestamp}"),
            filename,
            size_bytes: data.len() as u64,
            created_at: crate::db::models::now_iso8601(),
            status: "completed".to_string(),
        })
    }

    fn cleanup_old(&self, db_id: &str, keep: usize) {
        let dir = self.db_backup_dir(db_id);
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "gz"))
            .collect();

        entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

        for entry in entries.into_iter().skip(keep) {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    pub fn list_backups(&self, db_id: &str) -> Vec<BackupInfo> {
        let dir = self.db_backup_dir(db_id);
        let mut entries: Vec<BackupInfo> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "gz"))
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let name = e.file_name().to_string_lossy().to_string();
                Some(BackupInfo {
                    id: name.trim_end_matches(".sql.gz").to_string(),
                    filename: name,
                    size_bytes: meta.len(),
                    created_at: meta
                        .modified()
                        .ok()
                        .and_then(|t| {
                            let dur = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                            Some(
                                chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)?
                                    .to_rfc3339(),
                            )
                        })
                        .unwrap_or_default(),
                    status: "completed".to_string(),
                })
            })
            .collect();

        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries
    }

    pub fn get_backup_path(&self, db_id: &str, backup_id: &str) -> Option<PathBuf> {
        let dir = self.db_backup_dir(db_id);
        let path = dir.join(format!("{backup_id}.sql.gz"));
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
}

#[derive(Clone, serde::Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub status: String,
}

pub fn spawn_backup_scheduler(
    docker: Arc<DockerClient>,
    db: Arc<dyn Database>,
    backup_store: Arc<BackupStore>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;

            let Ok(managed_dbs) = db.list_managed_dbs().await else {
                continue;
            };

            for mdb in &managed_dbs {
                if mdb.backup_schedule.is_none() {
                    continue;
                }

                let container_name = format!("icefall-db-{}", mdb.name.to_lowercase());
                match backup_store
                    .run_backup(&docker, &mdb.id, &mdb.db_type, &container_name)
                    .await
                {
                    Ok(info) => {
                        tracing::info!(
                            "Backup completed for {} ({}): {}",
                            mdb.name,
                            mdb.db_type,
                            info.filename
                        );
                    }
                    Err(e) => {
                        tracing::error!("Backup failed for {}: {e}", mdb.name);
                    }
                }
            }
        }
    });
}
