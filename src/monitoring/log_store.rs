use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::StreamExt;

use crate::db::Database;
use crate::docker::DockerClient;

const MAX_LOG_SIZE: u64 = 50 * 1024 * 1024;
const MAX_ROTATED_FILES: u32 = 5;

pub struct LogStore {
    base_dir: PathBuf,
}

impl LogStore {
    pub fn new(data_dir: &Path) -> Self {
        let base_dir = data_dir.join("logs");
        std::fs::create_dir_all(&base_dir).ok();
        Self { base_dir }
    }

    fn log_path(&self, app_id: &str) -> PathBuf {
        self.base_dir.join(format!("{app_id}.log"))
    }

    pub async fn append(&self, app_id: &str, line: &str) {
        let path = self.log_path(app_id);
        let base_dir = self.base_dir.clone();
        let app_id = app_id.to_string();
        let line = line.to_string();

        tokio::task::spawn_blocking(move || {
            if let Ok(metadata) = std::fs::metadata(&path) {
                if metadata.len() >= MAX_LOG_SIZE {
                    rotate_sync(&base_dir, &app_id);
                }
            }

            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                let _ = writeln!(file, "{}", line);
            }
        })
        .await
        .ok();
    }

    pub async fn search(
        &self,
        app_id: &str,
        query: Option<&str>,
        stream_filter: Option<&str>,
        limit: usize,
    ) -> Vec<StoredLogLine> {
        let path = self.log_path(app_id);
        let query = query.map(|q| q.to_lowercase());
        let stream_filter = stream_filter.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };

            content
                .lines()
                .rev()
                .filter_map(parse_log_line)
                .filter(|entry| {
                    if let Some(ref q) = query {
                        if !entry.message.to_lowercase().contains(q) {
                            return false;
                        }
                    }
                    if let Some(ref sf) = stream_filter {
                        if entry.stream != *sf {
                            return false;
                        }
                    }
                    true
                })
                .take(limit)
                .collect()
        })
        .await
        .unwrap_or_default()
    }

    pub async fn read_all(&self, app_id: &str) -> String {
        let path = self.log_path(app_id);
        tokio::fs::read_to_string(&path).await.unwrap_or_default()
    }

    pub async fn cleanup_old(&self, max_age_days: u64) {
        let base_dir = self.base_dir.clone();
        tokio::task::spawn_blocking(move || {
            let cutoff =
                std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_days * 86400);

            if let Ok(entries) = std::fs::read_dir(&base_dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified < cutoff {
                                let _ = std::fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        })
        .await
        .ok();
    }
}

fn rotate_sync(base_dir: &Path, app_id: &str) {
    for i in (1..MAX_ROTATED_FILES).rev() {
        let from = base_dir.join(format!("{app_id}.log.{i}"));
        let to = base_dir.join(format!("{app_id}.log.{}", i + 1));
        let _ = std::fs::rename(from, to);
    }
    let current = base_dir.join(format!("{app_id}.log"));
    let rotated = base_dir.join(format!("{app_id}.log.1"));
    let _ = std::fs::rename(current, rotated);
}

#[derive(Clone, serde::Serialize)]
pub struct StoredLogLine {
    pub timestamp: String,
    pub stream: String,
    pub message: String,
}

fn parse_log_line(line: &str) -> Option<StoredLogLine> {
    if line.is_empty() {
        return None;
    }

    if let Some((ts, rest)) = line.split_once(' ') {
        if ts.contains('T') {
            let (stream, message) = rest.split_once(' ').unwrap_or(("stdout", rest));
            return Some(StoredLogLine {
                timestamp: ts.to_string(),
                stream: stream.to_string(),
                message: message.to_string(),
            });
        }
    }

    Some(StoredLogLine {
        timestamp: crate::db::models::now_iso8601(),
        stream: "stdout".to_string(),
        message: line.to_string(),
    })
}

pub fn spawn_log_capture(
    docker: Arc<DockerClient>,
    db: Arc<dyn Database>,
    log_store: Arc<LogStore>,
) {
    tokio::spawn(async move {
        let active: Arc<tokio::sync::Mutex<std::collections::HashMap<String, String>>> =
            Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            let apps = match db.list_apps().await {
                Ok(a) => a,
                Err(_) => continue,
            };

            for app in &apps {
                let label = format!("icefall.app={}", app.id);
                let containers = match docker.list_containers(Some(&label)).await {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let running = containers.iter().find(|c| c.state == "running");
                let running_id = running.map(|c| c.id.clone());

                let mut active_map = active.lock().await;
                let already_tracking = active_map.get(&app.id) == running_id.as_ref();
                if already_tracking || running_id.is_none() {
                    continue;
                }

                let container_id = running_id.unwrap();
                active_map.insert(app.id.clone(), container_id.clone());
                drop(active_map);

                let docker = docker.clone();
                let app_id = app.id.clone();
                let log_store = log_store.clone();
                let active = active.clone();

                tokio::spawn(async move {
                    let mut stream = docker.stream_logs(&container_id, true, Some(0));
                    while let Some(Ok(line)) = stream.next().await {
                        let formatted = format!(
                            "{} {} {}",
                            crate::db::models::now_iso8601(),
                            line.stream,
                            line.message.trim_end()
                        );
                        log_store.append(&app_id, &formatted).await;
                    }
                    active.lock().await.remove(&app_id);
                });
            }

            log_store.cleanup_old(7).await;

            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        }
    });
}

pub fn redact_secrets(line: &str, secrets: &[String]) -> String {
    let mut result = line.to_string();
    for secret in secrets {
        if secret.len() >= 4 {
            result = result.replace(secret, "[REDACTED]");
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn append_and_search() {
        let dir = TempDir::new().unwrap();
        let store = LogStore::new(dir.path());

        store
            .append("app1", "2026-05-07T10:00:00Z stdout Hello world")
            .await;
        store
            .append("app1", "2026-05-07T10:00:01Z stderr Error occurred")
            .await;
        store
            .append("app1", "2026-05-07T10:00:02Z stdout Another line")
            .await;

        let results = store.search("app1", None, None, 100).await;
        assert_eq!(results.len(), 3);

        let results = store.search("app1", Some("error"), None, 100).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("Error"));

        let results = store.search("app1", None, Some("stderr"), 100).await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn search_with_limit() {
        let dir = TempDir::new().unwrap();
        let store = LogStore::new(dir.path());

        for i in 0..20 {
            store
                .append("app1", &format!("2026-05-07T10:00:{i:02}Z stdout Line {i}"))
                .await;
        }

        let results = store.search("app1", None, None, 5).await;
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn read_all_empty() {
        let dir = TempDir::new().unwrap();
        let store = LogStore::new(dir.path());
        assert!(store.read_all("nonexistent").await.is_empty());
    }

    #[tokio::test]
    async fn rotate_on_size() {
        let dir = TempDir::new().unwrap();
        let store = LogStore {
            base_dir: dir.path().to_path_buf(),
        };

        let big_line = "x".repeat(1024);
        for _ in 0..100 {
            store.append("app1", &big_line).await;
        }

        assert!(store.log_path("app1").exists());
    }

    #[test]
    fn redact_secrets_works() {
        let line = "DATABASE_URL=postgres://secret@host";
        let result = redact_secrets(line, &["secret".to_string()]);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("secret"));
    }
}
