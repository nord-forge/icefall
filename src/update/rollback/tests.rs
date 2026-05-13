use super::*;
use crate::update::apply::PendingUpdate;
use tempfile::TempDir;

fn make_rollback(dir: &std::path::Path) -> UpdateRollback {
    UpdateRollback {
        data_dir: dir.to_path_buf(),
        binary_path: dir.join("fake-icefall"),
    }
}

#[test]
fn needs_rollback_returns_false_without_marker() {
    let tmp = TempDir::new().unwrap();
    let rb = make_rollback(tmp.path());
    assert!(!rb.needs_rollback());
}

#[test]
fn needs_rollback_returns_true_for_recent_marker() {
    let tmp = TempDir::new().unwrap();
    let marker_dir = tmp.path().join("updates");
    std::fs::create_dir_all(&marker_dir).unwrap();

    let marker = PendingUpdate {
        from_version: "0.1.0".into(),
        to_version: "0.2.0".into(),
        rollback_binary: "/tmp/rollback".into(),
        db_backup: "/tmp/backup.db".into(),
        dashboard_backup: None,
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    let json = serde_json::to_string_pretty(&marker).unwrap();
    std::fs::write(marker_dir.join("pending_update"), json).unwrap();

    let rb = make_rollback(tmp.path());
    assert!(rb.needs_rollback());
}

#[test]
fn needs_rollback_returns_false_for_old_marker() {
    let tmp = TempDir::new().unwrap();
    let marker_dir = tmp.path().join("updates");
    std::fs::create_dir_all(&marker_dir).unwrap();

    let old_time = chrono::Utc::now() - chrono::Duration::minutes(10);
    let marker = PendingUpdate {
        from_version: "0.1.0".into(),
        to_version: "0.2.0".into(),
        rollback_binary: "/tmp/rollback".into(),
        db_backup: "/tmp/backup.db".into(),
        dashboard_backup: None,
        started_at: old_time.to_rfc3339(),
    };
    let json = serde_json::to_string_pretty(&marker).unwrap();
    std::fs::write(marker_dir.join("pending_update"), json).unwrap();

    let rb = make_rollback(tmp.path());
    assert!(!rb.needs_rollback());
}

#[test]
fn has_rollback_detects_file() {
    let tmp = TempDir::new().unwrap();
    let rb = make_rollback(tmp.path());
    assert!(!rb.has_rollback());

    let rollback_dir = tmp.path().join("updates");
    std::fs::create_dir_all(&rollback_dir).unwrap();
    std::fs::write(rollback_dir.join("icefall.rollback"), b"binary").unwrap();
    assert!(rb.has_rollback());
}

#[test]
fn rollback_info_returns_metadata() {
    let tmp = TempDir::new().unwrap();
    let rollback_dir = tmp.path().join("updates");
    std::fs::create_dir_all(&rollback_dir).unwrap();
    std::fs::write(rollback_dir.join("icefall.rollback"), b"binary-data").unwrap();

    let rb = make_rollback(tmp.path());
    let info = rb.rollback_info().unwrap();
    assert_eq!(info.size_bytes, 11);
    assert!(info.modified_at.is_some());
}

#[test]
fn rollback_info_returns_none_when_missing() {
    let tmp = TempDir::new().unwrap();
    let rb = make_rollback(tmp.path());
    assert!(rb.rollback_info().is_none());
}

#[test]
fn execute_rollback_restores_binary_and_db() {
    let tmp = TempDir::new().unwrap();

    let binary_path = tmp.path().join("fake-icefall");
    std::fs::write(&binary_path, b"bad-binary").unwrap();

    let rollback_dir = tmp.path().join("rollback-store");
    std::fs::create_dir_all(&rollback_dir).unwrap();
    let rollback_binary = rollback_dir.join("icefall.rollback");
    std::fs::write(&rollback_binary, b"good-binary").unwrap();

    let backup_dir = tmp.path().join("backup-store");
    std::fs::create_dir_all(&backup_dir).unwrap();
    let db_backup = backup_dir.join("backup.db");
    std::fs::write(&db_backup, b"good-db-data").unwrap();

    let db_path = tmp.path().join("icefall.db");
    std::fs::write(&db_path, b"bad-db-data").unwrap();

    let marker_dir = tmp.path().join("updates");
    std::fs::create_dir_all(&marker_dir).unwrap();
    let marker = PendingUpdate {
        from_version: "0.1.0".into(),
        to_version: "0.2.0".into(),
        rollback_binary: rollback_binary.to_string_lossy().to_string(),
        db_backup: db_backup.to_string_lossy().to_string(),
        dashboard_backup: None,
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    let json = serde_json::to_string_pretty(&marker).unwrap();
    std::fs::write(marker_dir.join("pending_update"), &json).unwrap();

    let rb = UpdateRollback {
        data_dir: tmp.path().to_path_buf(),
        binary_path: binary_path.clone(),
    };

    rb.execute_rollback().unwrap();

    assert_eq!(std::fs::read(&binary_path).unwrap(), b"good-binary");
    assert_eq!(std::fs::read(&db_path).unwrap(), b"good-db-data");
    assert!(!marker_dir.join("pending_update").exists());
}

#[test]
fn execute_rollback_fails_without_marker() {
    let tmp = TempDir::new().unwrap();
    let rb = make_rollback(tmp.path());
    assert!(rb.execute_rollback().is_err());
}

#[test]
fn cleanup_old_rollbacks_removes_expired() {
    let tmp = TempDir::new().unwrap();
    let rollback_dir = tmp.path().join("updates");
    std::fs::create_dir_all(&rollback_dir).unwrap();
    let rollback_path = rollback_dir.join("icefall.rollback");
    std::fs::write(&rollback_path, b"old-binary").unwrap();

    let rb = make_rollback(tmp.path());

    rb.cleanup_old_rollbacks(0).unwrap();
    assert!(!rollback_path.exists());
}

#[test]
fn cleanup_old_rollbacks_keeps_recent() {
    let tmp = TempDir::new().unwrap();
    let rollback_dir = tmp.path().join("updates");
    std::fs::create_dir_all(&rollback_dir).unwrap();
    let rollback_path = rollback_dir.join("icefall.rollback");
    std::fs::write(&rollback_path, b"recent-binary").unwrap();

    let rb = make_rollback(tmp.path());

    rb.cleanup_old_rollbacks(30).unwrap();
    assert!(rollback_path.exists());
}
