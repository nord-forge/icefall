use super::*;
use tempfile::TempDir;

fn make_applier(dir: &std::path::Path) -> UpdateApplier {
    UpdateApplier {
        data_dir: dir.to_path_buf(),
        binary_path: dir.join("fake-icefall"),
    }
}

#[test]
fn pending_marker_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let applier = make_applier(tmp.path());

    assert!(applier.read_pending_marker().is_none());

    let marker = PendingUpdate {
        from_version: "0.1.0".into(),
        to_version: "0.2.0".into(),
        rollback_binary: "/tmp/rollback".into(),
        db_backup: "/tmp/backup.db".into(),
        dashboard_backup: None,
        started_at: "2026-05-10T12:00:00Z".into(),
    };
    applier.write_pending_marker(&marker).unwrap();

    let read = applier.read_pending_marker().unwrap();
    assert_eq!(read.from_version, "0.1.0");
    assert_eq!(read.to_version, "0.2.0");

    applier.clear_pending_marker().unwrap();
    assert!(applier.read_pending_marker().is_none());
}

#[test]
fn clear_missing_marker_is_ok() {
    let tmp = TempDir::new().unwrap();
    let applier = make_applier(tmp.path());
    assert!(applier.clear_pending_marker().is_ok());
}

#[test]
fn backup_binary_copies_file() {
    let tmp = TempDir::new().unwrap();
    let binary = tmp.path().join("fake-icefall");
    std::fs::write(&binary, b"binary-content").unwrap();

    let applier = UpdateApplier {
        data_dir: tmp.path().to_path_buf(),
        binary_path: binary,
    };

    let rollback_path = applier.backup_binary().unwrap();
    assert!(rollback_path.exists());
    assert_eq!(std::fs::read(&rollback_path).unwrap(), b"binary-content");
}

#[test]
fn swap_binary_replaces_file() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("icefall");
    std::fs::write(&target, b"old-binary").unwrap();

    let new_binary = tmp.path().join("new-icefall");
    std::fs::write(&new_binary, b"new-binary").unwrap();

    let applier = UpdateApplier {
        data_dir: tmp.path().to_path_buf(),
        binary_path: target.clone(),
    };

    applier.swap_binary(&new_binary).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"new-binary");

    assert!(!target.with_extension("new").exists());
}

#[test]
fn copy_dir_recursive_works() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("a.txt"), b"file-a").unwrap();
    std::fs::write(src.join("sub").join("b.txt"), b"file-b").unwrap();

    let dst = tmp.path().join("dst");
    super::copy_dir_recursive(&src, &dst).unwrap();

    assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"file-a");
    assert_eq!(
        std::fs::read(dst.join("sub").join("b.txt")).unwrap(),
        b"file-b"
    );
}

#[test]
fn pending_marker_deserializes_without_dashboard_backup() {
    let json = r#"{
        "from_version": "0.1.0",
        "to_version": "0.2.0",
        "rollback_binary": "/tmp/rb",
        "db_backup": "/tmp/db",
        "started_at": "2026-05-10T12:00:00Z"
    }"#;
    let marker: PendingUpdate = serde_json::from_str(json).unwrap();
    assert!(marker.dashboard_backup.is_none());
}
